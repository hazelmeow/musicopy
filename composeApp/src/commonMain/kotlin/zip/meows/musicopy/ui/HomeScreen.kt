package zip.meows.musicopy.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import io.github.alexzhirkevich.qrose.QrData
import io.github.alexzhirkevich.qrose.rememberQrCodePainter
import io.github.alexzhirkevich.qrose.text
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.compose_multiplatform
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.Model
import zip.meows.musicopy.Greeting

@Composable
fun HomeScreen(
    model: Model,
    onConnectQRButtonClicked: () -> Unit,
    onConnectManuallyButtonClicked: () -> Unit,
) {
    var showContent by remember { mutableStateOf(false) }
    Column(
        modifier = Modifier
            .safeContentPadding()
            .fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Button(onClick = { showContent = !showContent }) {
            Text("Click me!")
        }
        AnimatedVisibility(showContent) {
            val greeting = remember { Greeting().greet() }
            Column(
                Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Image(painterResource(Res.drawable.compose_multiplatform), null)
                Text("Compose: $greeting")
            }
        }

        val sheetState = rememberNodeStatusSheetState()
        Button(onClick = { sheetState.peek() }) {
            Text("Show Node Info")
        }

        Text("state = ${model}")

        model.node?.let {
            Image(
                painter = rememberQrCodePainter(
                    QrData.text(it.nodeId)
                ),
                contentDescription = "QR code containing node ID",
                modifier = Modifier.fillMaxWidth()
            )
        }

        Card() {
            Row() {

            }
        }

        Button(onClick = onConnectQRButtonClicked) {
            Text("connect qr")
        }
        Button(onClick = onConnectManuallyButtonClicked) {
            Text("connect manually")
        }

        NodeStatusSheet(sheetState, model)
    }
}

