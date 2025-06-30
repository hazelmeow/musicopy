package zip.meows.musicopy.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
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
import uniffi.musicopy.Model
import zip.meows.musicopy.shortenNodeId

@Composable
fun ConnectWidget(
    model: Model,
    onAcceptAndTrust: (remoteNodeId: String) -> Unit,
    onAcceptOnce: (remoteNodeId: String) -> Unit,
    onDeny: (remoteNodeId: String) -> Unit,
) {
    var nextPending = model.node.pendingConnections.firstOrNull()

    Card(
        modifier = Modifier.fillMaxWidth().aspectRatio(1f)
    ) {
        AnimatedContent(
            targetState = nextPending,
            transitionSpec = {
                // Compare the incoming number with the previous number.
                val targetConnectedAt = targetState?.connectedAt ?: 0u
                val initialConnectedAt = initialState?.connectedAt ?: 0u
                if (targetConnectedAt > initialConnectedAt) {
                    // If the target number is larger, it slides up and fades in
                    // while the initial (smaller) number slides up and fades out.
                    slideInVertically { height -> height } + fadeIn() togetherWith
                            slideOutVertically { height -> -height } + fadeOut()
                } else {
                    // If the target number is smaller, it slides down and fades in
                    // while the initial number slides down and fades out.
                    slideInVertically { height -> -height } + fadeIn() togetherWith
                            slideOutVertically { height -> height } + fadeOut()
                }.using(
                    // Disable clipping since the faded slide-in/out should
                    // be displayed out of bounds.
                    SizeTransform(clip = false)
                )
            },
        ) { targetState ->
            targetState?.let {
                PendingScreen(
                    remoteNodeId = targetState.nodeId,
                    remoteNodeName = targetState.name,
                    onAcceptAndTrust = { onAcceptAndTrust(targetState.nodeId) },
                    onAcceptOnce = { onAcceptOnce(targetState.nodeId) },
                    onDeny = { onAcceptOnce(targetState.nodeId) },
                )
            } ?: run {
                DefaultScreen(
                    localNodeId = model.node.nodeId,
                )
            }
        }
    }
}

@Composable
private fun DefaultScreen(
    localNodeId: String,
) {
    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("Connect", style = MaterialTheme.typography.titleLarge)

            Text("Lorem ipsum")

            Text("Download mobile app >")
        }

        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 16.dp),
            horizontalArrangement = Arrangement.Center
        ) {
            Image(
                painter = rememberQrCodePainter(
                    QrData.text(localNodeId)
                ),
                contentDescription = "QR code containing node ID",
                modifier = Modifier.widthIn(max = 100.dp)
            )
        }

        Row {
            Text("${localNodeId.slice(0..<6)}...")

            Text("copy btn")

            Box(modifier = Modifier.weight(1f))
        }
    }
}

@Composable
private fun PendingScreen(
    remoteNodeId: String,
    remoteNodeName: String,
    onAcceptAndTrust: () -> Unit,
    onAcceptOnce: () -> Unit,
    onDeny: () -> Unit,
) {
    var trust by remember { mutableStateOf(true) }

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Text("Pending connection", style = MaterialTheme.typography.titleLarge)

        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(remoteNodeName, style = MaterialTheme.typography.titleMedium)
            Text(shortenNodeId(remoteNodeId), style = MaterialTheme.typography.titleSmall)
        }

        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = {
                    if (trust) {
                        onAcceptAndTrust()
                    } else {
                        onAcceptOnce()
                    }
                }) {
                    Text("Allow")
                }
                OutlinedButton(onClick = onDeny) {
                    Text("Deny")
                }
            }

            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = trust, onCheckedChange = { trust = it })
                Text("Remember this device", style = MaterialTheme.typography.labelLarge)
            }
        }
    }
}
