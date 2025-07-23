package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.content_copy_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.Model
import zip.meows.musicopy.AppSettings
import zip.meows.musicopy.ui.NodeStatusSheet
import zip.meows.musicopy.ui.rememberNodeStatusSheetState

@Composable
fun HomeScreen(
    model: Model,
    onPickDownloadDirectory: () -> Unit,
    onConnectQRButtonClicked: () -> Unit,
    onConnectManuallyButtonClicked: () -> Unit,
) {
    Scaffold() { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(null)
            Button(
                onClick = onPickDownloadDirectory
            ) {
                Text("choose directory")
            }
            Text("download directory = ${downloadDirectory}")

            val sheetState = rememberNodeStatusSheetState()
            Button(onClick = { sheetState.peek() }) {
                Text("Show Node Info")
            }

            Column(
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Card(modifier = Modifier.height(50.dp), shape = CircleShape) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Text(
                            "Connect",
                            modifier = Modifier.padding(start = 12.dp),
                            style = MaterialTheme.typography.titleLarge
                        )

                        Box(modifier = Modifier.weight(1f))

                        Button(onClick = onConnectQRButtonClicked) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "QR code icon"
                            )
                            Text("QR")
                        }

                        Button(
                            onClick = onConnectManuallyButtonClicked,
                            shape = RoundedCornerShape(46.dp)
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "QR code icon"
                            )
                            Text("Manual")
                        }
                    }
                }
            }

            NodeStatusSheet(sheetState, model)
        }
    }
}

