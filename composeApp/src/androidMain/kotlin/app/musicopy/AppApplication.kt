package app.musicopy

import android.app.Application
import android.content.Context
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch

class AppApplication : Application() {
    lateinit var platformAppContext: PlatformAppContext
        private set

    lateinit var coreInstance: CoreInstance
        private set
    val coreInstanceReady = MutableStateFlow(false)

    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)

        // initialize ndk_context crate
        RustNdkContext.init(this)
    }

    override fun onCreate() {
        super.onCreate()

        // initialize platform app context
        platformAppContext = PlatformAppContext(this)

        // launch coroutine to initialize core instance asynchronously
        GlobalScope.launch {
            coreInstance = CoreInstance.start(platformAppContext)
            coreInstanceReady.value = true
        }
    }
}
