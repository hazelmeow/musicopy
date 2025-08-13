package app.musicopy

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import platform.UIKit.UIDevice
import platform.Foundation.NSNumber
import platform.Foundation.NSNumberFormatter

actual class PlatformContext actual constructor() {
    actual val name: String =
        UIDevice.currentDevice.systemName() + " " + UIDevice.currentDevice.systemVersion
}

@OptIn(ExperimentalComposeUiApi::class)
actual fun toClipEntry(string: String): ClipEntry = ClipEntry.withPlainText(string)

actual object CoreProvider : ICoreProvider

actual fun formatFloat(f: Float, decimals: Int): String {
    val formatter = NSNumberFormatter()
    formatter.minimumFractionDigits = 0u
    formatter.maximumFractionDigits = decimals.toULong()
    formatter.numberStyle = 1u // Decimal
    return formatter.stringFromNumber(NSNumber(f))!!
}
