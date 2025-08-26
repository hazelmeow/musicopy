package app.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.awaitApplication
import androidx.compose.ui.window.rememberWindowState
import app.musicopy.ui.DesktopApp
import com.composeunstyled.Text
import kotlinx.coroutines.runBlocking


const val WINDOW_WIDTH = 800
const val WINDOW_HEIGHT = 600

fun main() = runBlocking {
    val platformAppContext = PlatformAppContext()

    // TODO: measure how long blocking on this takes
    // TODO: maybe switch to splash screen of some sort
    val coreInstance = CoreInstance.start(platformAppContext)

    awaitApplication {
        val state = rememberWindowState(
            size = DpSize(WINDOW_WIDTH.dp, WINDOW_HEIGHT.dp),
        )

        Window(
            title = "Musicopy",
            // TODO: seems to maybe be broken after switching to awaitApplication for async setup
            onCloseRequest = ::exitApplication,
            state = state
        ) {
//        window.minimumSize = Dimension(800, 600)

            val platformActivityContext = PlatformActivityContext(mainWindow = window)

            DesktopApp(
                platformAppContext = platformAppContext,
                platformActivityContext = platformActivityContext,
                coreInstance = coreInstance
            )

            // TODO
            Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
                Text("window: ${LocalWindowInfo.current.containerSize}")
            }
        }
    }
}
