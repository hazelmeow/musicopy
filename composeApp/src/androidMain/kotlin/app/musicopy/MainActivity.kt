package app.musicopy

import android.content.ContentResolver
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner

class AppLifecycleObserver(
    private val registry: ActivityResultRegistry,
    private val contentResolver: ContentResolver,
) :
    DefaultLifecycleObserver {
    lateinit var openDocumentTree: ActivityResultLauncher<Uri?>

    override fun onCreate(owner: LifecycleOwner) {
        openDocumentTree =
            registry.register("key", owner, ActivityResultContracts.OpenDocumentTree()) { uri ->
                if (uri == null) {
                    // TODO
                    return@register
                }

                // persist permission
                val modeFlags = Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                contentResolver.takePersistableUriPermission(uri, modeFlags)

                // store
                AppSettings.downloadDirectory = uri.toString()
            }
    }
}

class MainActivity : ComponentActivity() {
    lateinit var observer: AppLifecycleObserver

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()

        val splashScreen = installSplashScreen()

        super.onCreate(savedInstanceState)

        // register activity lifecycle observer
        observer = AppLifecycleObserver(activityResultRegistry, contentResolver)
        lifecycle.addObserver(observer)

        // wait for core instance to be initialized
        val app = application as AppApplication
        splashScreen.setKeepOnScreenCondition { !app.coreInstanceReady.value }

        val platformActivityContext = PlatformActivityContext(this)

        setContent {
            val coreInstanceReady by app.coreInstanceReady.collectAsState()
            if (coreInstanceReady) {
                App(
                    platformAppContext = app.platformAppContext,
                    platformActivityContext = platformActivityContext,
                    coreInstance = app.coreInstance
                )
            }
        }
    }
}
