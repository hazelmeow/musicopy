package zip.meows.musicopy

import android.content.ClipData
import android.icu.text.DecimalFormat
import android.os.Build
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.toClipEntry

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

actual object CoreProvider : ICoreProvider

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}
