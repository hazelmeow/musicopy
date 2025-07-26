package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import uniffi.musicopy.ClientModel
import zip.meows.musicopy.shortenNodeId
import zip.meows.musicopy.ui.components.SectionCard
import zip.meows.musicopy.ui.components.TopBar

@Composable
fun WaitingScreen(
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Waiting to connect",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            verticalArrangement = Arrangement.Center,
        ) {
            SectionCard(
                title = "Waiting to connect",
                body = {
                    Text("Connected to ${shortenNodeId(clientModel.nodeId)}")

                    Text("Press Accept on the other device to continue.")
                },
                onCancel = onCancel,
            )
        }
    }
}
