package app.mcadmin.mobile

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Android 13+: разрешение на уведомления о событиях серверов
    if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
      requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
    }
  }

  // настройки фоновой службы меняются в приложении — синхронизируем при сворачивании и возврате
  override fun onStop() { super.onStop(); PanelService.sync(this) }
  override fun onResume() { super.onResume(); PanelService.sync(this) }
}
