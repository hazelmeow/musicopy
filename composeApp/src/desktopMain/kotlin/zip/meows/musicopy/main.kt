package zip.meows.musicopy

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application

fun main() = application {
    Window(
        onCloseRequest = ::exitApplication,
        title = "Musicopy",
    ) {
        App()
    }
}