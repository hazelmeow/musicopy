package app.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import com.composeunstyled.Text

fun main() = application {
    val state = rememberWindowState()

    Window(
        title = "Musicopy",
        onCloseRequest = ::exitApplication,
        state = state
    ) {
        val platformContext = PlatformContext(mainWindow = window)

        App(platformContext)

        // TODO
        Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
            Text("window: ${LocalWindowInfo.current.containerSize}")
        }
    }
}
