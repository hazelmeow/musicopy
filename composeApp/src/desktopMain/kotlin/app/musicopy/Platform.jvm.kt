package app.musicopy

import androidx.compose.ui.platform.ClipEntry
import uniffi.musicopy.CoreOptions
import java.awt.Window
import java.awt.datatransfer.StringSelection
import java.text.DecimalFormat

actual class PlatformAppContext actual constructor() {
    actual val name: String = "Java ${System.getProperty("java.version")}"
}

actual class PlatformActivityContext private actual constructor() {
    lateinit var mainWindow: Window
        private set

    constructor(mainWindow: Window) : this() {
        this.mainWindow = mainWindow
    }
}

actual object CoreProvider : ICoreProvider {
    override fun getOptions(platformAppContext: PlatformAppContext): CoreOptions {
        val defaults = super.getOptions(platformAppContext)
//        defaults.inMemory = true
        return defaults
    }
}

actual fun toClipEntry(string: String): ClipEntry = ClipEntry(StringSelection(string))

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}
