package zip.meows.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.chevron_forward_24px
import musicopy.composeapp.generated.resources.content_copy_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import zip.meows.musicopy.ui.components.SectionCard

@Composable
fun PreTransferScreen(
    clientModel: ClientModel,
    onDownloadAll: () -> Unit,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
    ) {
        SectionCard(
            title = "Transfer",
            body = {

                Text("choose what to download")

                clientModel.index?.let { index ->
                    Text("received index of ${index.size} items")
                } ?: run {
                    Text("no index yet")
                }
            },
            onCancel = onCancel,
        )

        Card(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
            onClick = onDownloadAll,
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Download everything", style = MaterialTheme.typography.titleMedium)

                Icon(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = null,
                )
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
            onClick = { print("meow") }
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Choose what to download", style = MaterialTheme.typography.titleMedium)

                Icon(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = null
                )
            }
        }
    }
}
