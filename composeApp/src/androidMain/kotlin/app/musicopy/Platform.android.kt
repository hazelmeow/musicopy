package app.musicopy

import android.content.ClipData
import android.icu.text.DecimalFormat
import android.os.Build
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.toClipEntry
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.ProjectDirsOptions

actual class PlatformAppContext private actual constructor() {
    actual val name: String = "Android ${Build.VERSION.SDK_INT}"

    lateinit var application: AppApplication

    constructor(application: AppApplication) : this() {
        this.application = application
    }
}

actual class PlatformActivityContext private actual constructor() {
    lateinit var mainActivity: MainActivity
        private set

    constructor(mainActivity: MainActivity) : this() {
        this.mainActivity = mainActivity
    }
}

actual object CoreProvider : ICoreProvider {
    override fun getOptions(platformAppContext: PlatformAppContext): CoreOptions {
        val options = super.getOptions(platformAppContext)
        options.projectDirs = ProjectDirsOptions(
            dataDir = platformAppContext.application.filesDir.path,
            cacheDir = platformAppContext.application.cacheDir.path
        )
        return options
    }
}

actual fun toClipEntry(string: String): ClipEntry =
    ClipData.newPlainText("label", string).toClipEntry()

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}
