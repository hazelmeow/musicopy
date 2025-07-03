package zip.meows.musicopy

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import platform.UIKit.UIDevice

actual class PlatformContext actual constructor() {
    actual val name: String =
        UIDevice.currentDevice.systemName() + " " + UIDevice.currentDevice.systemVersion
}

@OptIn(ExperimentalComposeUiApi::class)
actual fun toClipEntry(string: String): ClipEntry = ClipEntry.withPlainText(string)
