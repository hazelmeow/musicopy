package zip.meows.musicopy

import androidx.compose.ui.platform.ClipEntry
import java.awt.Window
import java.awt.datatransfer.StringSelection

actual class PlatformContext private actual constructor() {
    actual val name: String = "Java ${System.getProperty("java.version")}"

    lateinit var mainWindow: Window
        private set

    constructor(mainWindow: Window) : this() {
        this.mainWindow = mainWindow
    }
}

actual fun toClipEntry(string: String): ClipEntry = ClipEntry(StringSelection(string))

actual object CoreProvider : ICoreProvider
