package zip.meows.musicopy

import android.content.ClipData
import android.os.Build
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.toClipEntry

class AndroidPlatform : Platform {
    override val name: String = "Android ${Build.VERSION.SDK_INT}"
}

actual fun getPlatform(): Platform = AndroidPlatform()

actual fun toClipEntry(string: String): ClipEntry = ClipData.newPlainText("label", string).toClipEntry()
