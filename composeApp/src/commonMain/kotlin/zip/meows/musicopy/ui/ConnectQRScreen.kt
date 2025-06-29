package zip.meows.musicopy.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import io.github.alexzhirkevich.qrose.QrData
import io.github.alexzhirkevich.qrose.rememberQrCodePainter
import io.github.alexzhirkevich.qrose.text
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.compose_multiplatform
import org.jetbrains.compose.resources.painterResource
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.musicopy.Model
import zip.meows.musicopy.Greeting

@Composable
fun ConnectQRScreen(
    onScan: (String) -> Unit,
) {
    var showContent by remember { mutableStateOf(false) }
    Column(
        modifier = Modifier
            .fillMaxSize().padding(horizontal = 8.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(8.dp, 16.dp)) {
                Text("Connect using QR code", style = MaterialTheme.typography.titleLarge)

                Text("Scan the QR code etc etc etc.")

                QRScanner(onResult = { nodeId ->
                    onScan(nodeId)
                })
            }
        }
    }
}
