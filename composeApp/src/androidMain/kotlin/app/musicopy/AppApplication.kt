package app.musicopy

import android.app.Application
import android.content.Context

class AppApplication : Application() {
    lateinit var platformAppContext: PlatformAppContext
        private set

    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)

        // initialize ndk_context crate
        RustNdkContext.init(this)
    }

    override fun onCreate() {
        super.onCreate()

        // initialize platform app context
        platformAppContext = PlatformAppContext(this)
    }
}
