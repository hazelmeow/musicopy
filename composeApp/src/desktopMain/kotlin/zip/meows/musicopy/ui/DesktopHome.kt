package zip.meows.musicopy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.dp
import uniffi.musicopy.Model

@Composable
fun DesktopHome(
    model: Model,
    onAcceptAndTrust: (remoteNodeId: String) -> Unit,
    onAcceptOnce: (remoteNodeId: String) -> Unit,
    onDeny: (remoteNodeId: String) -> Unit,
    onAddLibraryRoot: (name: String, path: String) -> Unit,
    onRemoveLibraryRoot: (name: String) -> Unit,
) {
    val oneCol = LocalWindowInfo.current.containerSize.width < 600

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Column(
            modifier = Modifier.widthIn(0.dp, 840.dp).padding(32.dp)
        ) {
            Row {
                Text("Musicopy")
                Box(modifier = Modifier.weight(1f))
                Text("?")
            }

            val left = @Composable {
                ConnectWidget(
                    model = model,
                    onAcceptAndTrust = onAcceptAndTrust,
                    onAcceptOnce = onAcceptOnce,
                    onDeny = onDeny,
                )
                LibraryWidget(
                    model = model,
                    onAddRoot = onAddLibraryRoot,
                    onRemoveRoot = onRemoveLibraryRoot
                )
            }
            val right = @Composable {
                JobsWidget(
                    model = model,
                )
            }

            if (oneCol) {
                Column(
                    modifier = Modifier.fillMaxSize(),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    left()
                    right()
                }
            } else {
                Row(
                    modifier = Modifier.fillMaxSize(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Column(
                        modifier = Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        left()
                    }
                    Column(
                        modifier = Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        right()
                    }
                }
            }
        }
    }
}