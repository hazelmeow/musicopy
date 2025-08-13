package app.musicopy

import android.app.Application
import android.content.Context

class AppApplication : Application() {
    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)

        // initialize ndk_context crate
        RustNdkContext.init(this)
    }
}
