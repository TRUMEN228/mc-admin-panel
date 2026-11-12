package app.mcadmin.mobile

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Фоновая служба: держит WebSocket с MC Admin Panel и показывает уведомления
 * (сбой сервера, заход игрока, ошибка задачи). Настройки пишет ядро приложения в notify.json.
 */
class PanelService : Service() {
    companion object {
        const val CH_STATUS = "status"
        const val CH_EVENTS = "events"
        private const val ID_FG = 1

        fun configFile(ctx: Context): File? =
            listOf(File(ctx.dataDir, "notify.json"), File(ctx.filesDir, "notify.json"))
                .firstOrNull { it.exists() }

        fun config(ctx: Context): JSONObject? = try {
            configFile(ctx)?.readText()?.let { JSONObject(it) }?.takeIf { it.optString("token").isNotEmpty() }
        } catch (e: Exception) { null }

        /** Запустить службу, если привязана панель и включена работа в фоне; иначе остановить. */
        fun sync(ctx: Context) {
            val i = Intent(ctx, PanelService::class.java)
            if (config(ctx) != null) {
                if (Build.VERSION.SDK_INT >= 26) ctx.startForegroundService(i) else ctx.startService(i)
            } else ctx.stopService(i)
        }
    }

    private val main = Handler(Looper.getMainLooper())
    private val http = OkHttpClient.Builder().connectTimeout(6, TimeUnit.SECONDS).pingInterval(30, TimeUnit.SECONDS).readTimeout(0, TimeUnit.SECONDS).build()
    private var ws: WebSocket? = null
    private var cfg: JSONObject? = null
    private var addrIndex = 0
    private var delay = 2L
    private var online = false
    private var notifId = 100
    private val names = HashMap<String, String>()

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        val nm = getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) {
            nm.createNotificationChannel(NotificationChannel(CH_STATUS, "Связь с панелью", NotificationManager.IMPORTANCE_MIN))
            nm.createNotificationChannel(NotificationChannel(CH_EVENTS, "События серверов", NotificationManager.IMPORTANCE_HIGH))
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        cfg = config(this)
        if (cfg == null) { stopSelf(); return START_NOT_STICKY }
        val n = statusNotification("Подключение к «${cfg!!.optString("name")}»…")
        if (Build.VERSION.SDK_INT >= 34) startForeground(ID_FG, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(ID_FG, n)
        ws?.close(1000, null)
        ws = null
        addrIndex = 0
        connect()
        return START_STICKY
    }

    override fun onDestroy() {
        ws?.close(1000, null)
        main.removeCallbacksAndMessages(null)
        super.onDestroy()
    }

    private fun openApp(): PendingIntent = PendingIntent.getActivity(this, 0,
        Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)

    private fun statusNotification(text: String): Notification {
        val b = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(this, CH_STATUS) else @Suppress("DEPRECATION") Notification.Builder(this)
        return b.setSmallIcon(R.mipmap.ic_launcher).setContentTitle("MC Admin Mobile").setContentText(text)
            .setOngoing(true).setContentIntent(openApp()).build()
    }

    private fun setStatus(text: String) {
        getSystemService(NotificationManager::class.java).notify(ID_FG, statusNotification(text))
    }

    private fun notifyEvent(title: String, text: String) {
        val b = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(this, CH_EVENTS) else @Suppress("DEPRECATION") Notification.Builder(this)
        val n = b.setSmallIcon(R.mipmap.ic_launcher).setContentTitle(title).setContentText(text)
            .setStyle(Notification.BigTextStyle().bigText(text)).setAutoCancel(true).setContentIntent(openApp()).build()
        getSystemService(NotificationManager::class.java).notify(notifId++, n)
    }

    private fun connect() {
        val c = cfg ?: return
        val addrs = c.optJSONArray("addresses") ?: return
        if (addrs.length() == 0) return
        val addr = addrs.getString(addrIndex % addrs.length())
        val url = "ws://$addr:${c.optInt("port", 7777)}/api/events?token=${c.optString("token")}&types=crash,players,job,status"
        ws = http.newWebSocket(Request.Builder().url(url).build(), object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                main.post {
                    delay = 2; online = true
                    setStatus("На связи с «${c.optString("name")}»")
                }
            }
            override fun onMessage(webSocket: WebSocket, text: String) { main.post { handle(text) } }
            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) { main.post { retry(webSocket, response?.code == 401) } }
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { main.post { retry(webSocket, false) } }
        })
    }

    private fun retry(socket: WebSocket, unauthorized: Boolean) {
        if (socket !== ws) return
        ws = null
        if (unauthorized) { setStatus("Доступ к панели отозван — привяжите телефон заново"); return }
        if (online) { online = false; setStatus("Нет связи с панелью — переподключение…") }
        addrIndex++
        main.postDelayed({ if (ws == null) connect() }, delay * 1000)
        delay = minOf(delay * 2, 60)
    }

    private fun handle(text: String) {
        val c = cfg ?: return
        val m = try { JSONObject(text) } catch (e: Exception) { return }
        val id = m.optString("id")
        if (m.has("name") && id.isNotEmpty()) names[id] = m.optString("name")
        val srv = names[id] ?: "сервер"
        when (m.optString("type")) {
            "crash" -> if (c.optBoolean("crash", true)) {
                val hint = m.optJSONObject("diag")?.optJSONArray("hints")?.optJSONObject(0)
                notifyEvent("Сбой: ${m.optString("name", srv)}", hint?.let { "${it.optString("title")}. ${it.optString("text")}" } ?: "Код выхода ${m.optInt("code")}")
            }
            "players" -> if (c.optBoolean("players", true) && m.optBoolean("joined")) {
                notifyEvent("${m.optString("player")} зашёл на сервер", "«${m.optString("name", srv)}» · онлайн: ${m.optJSONArray("players")?.length() ?: 0}")
            }
            "job" -> if (c.optBoolean("jobs", true)) {
                val j = m.optJSONObject("job") ?: return
                if (j.optString("status") == "error") {
                    val log = j.optJSONArray("log")
                    notifyEvent("Ошибка: ${j.optString("title")}", if (log != null && log.length() > 0) log.getString(log.length() - 1) else "")
                }
            }
        }
    }
}

/** После перезагрузки телефона служба поднимается сама. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) PanelService.sync(context)
    }
}
