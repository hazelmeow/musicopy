package app.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import com.composeunstyled.Text
import app.musicopy.ui.screens.PreTransferScreenSandbox

fun main() = application {
    val state = rememberWindowState(
        size = DpSize(WINDOW_WIDTH.dp, WINDOW_HEIGHT.dp),
    )

    Window(
        title = "Musicopy [Sandbox]",
        onCloseRequest = ::exitApplication,
        state = state
    ) {
        val platformContext = PlatformContext(mainWindow = window)

        Sandbox(platformContext)

        // TODO
        Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
            Text("window: ${LocalWindowInfo.current.containerSize}")
        }
    }
}

@Composable
private fun Sandbox(
    platformContext: PlatformContext,
) {
    MaterialTheme {
        SandboxContent()
    }
}

@Composable
private fun SandboxContent() {
    PreTransferScreenSandbox()
}
