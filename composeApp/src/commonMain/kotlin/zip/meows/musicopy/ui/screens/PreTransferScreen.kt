package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import zip.meows.musicopy.ui.components.DetailBox
import zip.meows.musicopy.ui.components.DetailItem
import zip.meows.musicopy.ui.components.TopBar

@Composable
fun PreTransferScreen(
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    onDownloadAll: () -> Unit,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Transfer",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
        ) {
            Column(
                modifier = Modifier.padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                DetailBox {
                    DetailItem("Folders", "123")
                    DetailItem("Files", "456")
                    DetailItem("Total Size", "4.27 GB")
                }

                Button(
                    onClick = onDownloadAll,
                    modifier = Modifier.fillMaxWidth().height(64.dp),
                    shape = MaterialTheme.shapes.large,
                    contentPadding = PaddingValues(16.dp)
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Download everything")

                        Icon(
                            painter = painterResource(Res.drawable.chevron_forward_24px),
                            contentDescription = null,
                        )
                    }
                }
            }

            HorizontalDivider(thickness = 1.dp)

            HomeSection(
                title = "FILES"
            ) {
                Text("asdf")
                Text("asdf")
                Text("asdf")
                Text("asdf")
                Text("asdf")
            }
        }
    }
}
