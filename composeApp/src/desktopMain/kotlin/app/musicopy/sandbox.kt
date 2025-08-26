package app.musicopy

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import app.musicopy.ui.Theme
import app.musicopy.ui.components.Info
import app.musicopy.ui.screens.DisconnectedScreenSandbox
import app.musicopy.ui.screens.HomeScreenSandbox
import com.composeunstyled.Text
import app.musicopy.ui.screens.PreTransferScreenSandbox
import app.musicopy.ui.screens.TransferScreenSandbox
import app.musicopy.ui.screens.WaitingScreenSandbox
import app.musicopy.ui.screenshots.DesktopHomeScreenshot
import app.musicopy.ui.screenshots.MobileTransferScreenshot
import io.github.alexzhirkevich.qrose.toByteArray
import kotlinx.coroutines.launch
import java.io.File

fun main() = application {
    val state = rememberWindowState(
        size = DpSize(WINDOW_WIDTH.dp, WINDOW_HEIGHT.dp),
    )

    Window(
        title = "Musicopy [Sandbox]",
        onCloseRequest = ::exitApplication,
        state = state,
    ) {
        val platformAppContext = PlatformAppContext()
        val platformActivityContext = PlatformActivityContext(mainWindow = window)

        Sandbox()

        // TODO
        Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
            Text("window: ${LocalWindowInfo.current.containerSize}")
        }
    }
}

@Composable
private fun Sandbox() {
    Theme {
        SandboxContent()
    }
}

@Composable
private fun SandboxContent() {
    WaitingScreenSandbox()
}

@Composable
fun SandboxScreenshot() {
    val isMobile = true

    val width = if (isMobile) 350 else WINDOW_WIDTH
    val height = WINDOW_HEIGHT

    Screenshot(
        width = width,
        height = height,
    ) {
//        DesktopHomeScreenshot()
        MobileTransferScreenshot()
    }
}

@Composable
private fun Screenshot(
    width: Int,
    height: Int,
    content: @Composable () -> Unit,
) {
    val coroutineScope = rememberCoroutineScope()
    val graphicsLayer = rememberGraphicsLayer()

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Info {
            Text(
                text = "Screenshot size: $width x $height",
                style = MaterialTheme.typography.bodyMedium
            )

            OutlinedButton(
                onClick = {
                    coroutineScope.launch {
                        val bitmap = graphicsLayer.toImageBitmap()
                        val bytes = bitmap.toByteArray()
                        File("./screenshot_home.png").writeBytes(bytes)
                    }
                }
            ) {
                Text("Screenshot")
            }
        }

        Box(
            modifier = Modifier
                .drawWithContent {
                    graphicsLayer.record {
                        this@drawWithContent.drawContent()
                    }
                    drawLayer(graphicsLayer)
                }
        ) {
            Box(
                modifier = Modifier
                    .size(width = width.dp, height = height.dp)
                    .background(MaterialTheme.colorScheme.surface)
            ) {
                content()
            }
        }
    }
}
