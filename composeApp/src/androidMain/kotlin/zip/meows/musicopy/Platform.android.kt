package zip.meows.musicopy

import android.content.ClipData
import android.icu.text.DecimalFormat
import android.os.Build
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.toClipEntry
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.ProjectDirsOptions

actual class PlatformContext private actual constructor() {
    actual val name: String = "Android ${Build.VERSION.SDK_INT}"

    lateinit var mainActivity: MainActivity
        private set

    constructor(mainActivity: MainActivity) : this() {
        this.mainActivity = mainActivity
    }
}

actual fun toClipEntry(string: String): ClipEntry =
    ClipData.newPlainText("label", string).toClipEntry()

actual object CoreProvider : ICoreProvider {
    override fun getOptions(platformContext: PlatformContext): CoreOptions {
        val options = super.getOptions(platformContext)
        options.projectDirs = ProjectDirsOptions(
            dataDir = platformContext.mainActivity.filesDir.path,
            cacheDir = platformContext.mainActivity.cacheDir.path
        )
        return options
    }
}

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}
