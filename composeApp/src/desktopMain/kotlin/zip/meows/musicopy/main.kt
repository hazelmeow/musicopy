package zip.meows.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.WindowPlacement
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import com.composeunstyled.Text
import zip.meows.musicopy.ui.DesktopApp
import java.awt.Dimension


const val WINDOW_WIDTH = 800
const val WINDOW_HEIGHT = 600

fun main() = application {
    val state = rememberWindowState(
        size = DpSize(WINDOW_WIDTH.dp, WINDOW_HEIGHT.dp),
    )

    Window(
        title = "Musicopy",
        onCloseRequest = ::exitApplication,
        state = state
    ) {
//        window.minimumSize = Dimension(800, 600)

//        App()
        DesktopApp()

        Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
            Text("window: ${LocalWindowInfo.current.containerSize}")
        }
    }
}
