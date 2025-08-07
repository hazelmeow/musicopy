package zip.meows.musicopy.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TriStateCheckbox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.fastJoinToString
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemModel
import zip.meows.musicopy.formatFloat
import zip.meows.musicopy.mockClientModel
import zip.meows.musicopy.ui.components.DetailBox
import zip.meows.musicopy.ui.components.DetailItem
import zip.meows.musicopy.ui.components.SectionHeader
import zip.meows.musicopy.ui.components.TopBar

@Composable
fun PreTransferScreen(
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    onDownloadAll: () -> Unit,
    onCancel: () -> Unit,
) {
    val numFolders by remember {
        derivedStateOf {
            countIndexFolders(
                clientModel.index ?: emptyList()
            )
        }
    }
    val numFiles by remember {
        derivedStateOf {
            clientModel.index?.size ?: 0
        }
    }
    val totalSize by remember {
        derivedStateOf {
            clientModel.index?.let { index ->
                index.sumOf { item -> item.fileSize.value() }
            } ?: 0u
        }
    }
    // display ~ if any size is estimated or unknown
    val totalSizeEstimated by remember {
        derivedStateOf {
            clientModel.index?.let { index ->
                index.any { it.fileSize !is FileSizeModel.Actual }
            } ?: false
        }
    }
    val totalSizeGB = totalSize.toFloat() / 1_000_000_000f

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
                    DetailItem("Folders", "$numFolders")
                    DetailItem("Files", "$numFiles")
                    DetailItem(
                        "Total Size", "${
                            if (totalSizeEstimated) {
                                "~"
                            } else {
                                ""
                            }
                        }${formatFloat(totalSizeGB, 1)} GB"
                    )
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

            SectionHeader("FILES")

            Tree(clientModel = clientModel)
        }
    }
}

@Composable
internal fun Tree(clientModel: ClientModel) {
    // build node graph
    val topLevelNodes by remember {
        derivedStateOf {
            buildTree(clientModel.index ?: emptyList())
        }
    }

    // build node size lookup
    val nodeSizes by remember {
        derivedStateOf {
            buildNodeSizes(topLevelNodes)
        }
    }

    val expanded = remember {
        val expanded = mutableStateListOf<TreeNode>()

        // expand top level nodes if they contain only non-leaves
        for (node in topLevelNodes) {
            if (node.children.all { child -> child.leaf == null }) {
                expanded.add(node)
            }
        }

        expanded
    }

    val selected = remember { mutableStateListOf<IndexItemModel>() }

    LazyColumn {
        topLevelNodes.forEach { topLevelNode ->
            renderNode(
                node = topLevelNode,
                isExpanded = { node ->
                    expanded.contains(node)
                },
                onExpand = { node ->
                    if (expanded.contains(node)) {
                        expanded.remove(node)
                    } else {
                        expanded.add(node)
                    }
                },
                isSelected = { item -> selected.contains(item) },
                onSelect = { item, shouldSelect ->
                    if (shouldSelect) {
                        selected.add(item)
                    } else {
                        selected.remove(item)
                    }
                },
                nodeSizes = nodeSizes
            )
        }
    }
}

/**
 * Builds the graph of `TreeNodes` from the index.
 *
 * Returns a list of top-level nodes.
 */
internal fun buildTree(
    index: List<IndexItemModel>,
): List<TreeNode> {
    val roots = mutableListOf<TreeNode>()

    // add nodes to tree
    for (item in index) {
        // find or create root
        val root = roots.find { node -> node.part == item.root } ?: run {
            val new = TreeNode(
                part = item.root,
            )
            roots.add(new)
            new
        }

        // split into path parts and filename
        val path = item.path.removePrefix("/")
        val parts = path.split('/')
        val lastPart = parts.last()
        val pathParts = parts.dropLast(1)

        // recursively find or create path nodes
        var curr = root
        for (part in pathParts) {
            val next = curr.children.find { node -> node.part == part } ?: run {
                val new = TreeNode(
                    part = part,
                )
                curr.children.add(new)
                new
            }
            curr = next
        }

        // create leaf node
        curr.children.add(
            TreeNode(
                part = lastPart,
                leaf = item
            )
        )
    }

    // collapse nodes with no loose files
    for (root in roots) {
        collapseNodeChildren(root)
    }

    // if there's only one root, return its contents as the top level
    return if (roots.size == 1) {
        roots[0].children
    } else {
        roots
    }
}

/**
 * Collapses the children of a `TreeNode` recursively.
 */
internal fun collapseNodeChildren(node: TreeNode) {
    // recursively collapse children first
    for (child in node.children) {
        collapseNodeChildren(child)
    }

    // duplicate list so we can safely iterate while modifying
    val oldChildren = node.children.toList()

    for (child in oldChildren) {
        // can't collapse leaves
        if (child.leaf != null) {
            continue;
        }

        // only collapse if all grandchildren are non-leafs
        val shouldCollapse = child.children.all { grandchild -> grandchild.leaf == null }
        if (!shouldCollapse) {
            continue
        }

        // find index to insert at
        val childIndex = node.children.indexOf(child)

        // add grandchildren with combined path to parent node
        // reverse iterator so the added nodes are in the correct order
        for (grandchild in child.children.reversed()) {
            val newNode = TreeNode(
                part = "${child.part}/${grandchild.part}",
                children = grandchild.children,
                leaf = grandchild.leaf,
            )
            node.children.add(childIndex, newNode)
        }

        // remove this node from the parent node
        node.children.remove(child)
    }
}

/**
 * Builds a map of sizes of TreeNodes.
 */
internal fun buildNodeSizes(
    nodes: List<TreeNode>,
    map: MutableMap<TreeNode, FileSizeModel> = mutableMapOf(),
): MutableMap<TreeNode, FileSizeModel> {
    for (node in nodes) {
        // recursively build sizes of children
        buildNodeSizes(node.children, map)

        // determine size of this node
        val size = node.leaf?.fileSize ?: run {
            // internal node's size is sum of child sizes
            val total = node.children.sumOf { child ->
                val childSize = map.getOrElse(
                    child,
                    defaultValue = { FileSizeModel.Unknown }
                )
                childSize.value()
            }

            // internal node is estimated if any child size is not actual
            val isEstimated = node.children.any { child ->
                val childSize = map.getOrElse(
                    child,
                    defaultValue = { FileSizeModel.Unknown }
                )
                childSize !is FileSizeModel.Actual
            }

            if (isEstimated) {
                FileSizeModel.Estimated(total)
            } else {
                FileSizeModel.Actual(total)
            }
        }

        // add to map
        map[node] = size
    }

    return map
}

/**
 * Gets the `ToggleableState` of a node in the file tree.
 *
 * If the node is a leaf (file), the state is on if selected and off if not selected.
 * If the node is a branch, then:
 *  - If it has no children, it is null
 *  - If any child is indeterminate, it is indeterminate
 *  - If one child is on and another is off, it is indeterminate
 *  - If any child is on, it is on (note that all children are on)
 *  - Otherwise, it is off
 */
internal fun getNodeState(
    node: TreeNode,
    isSelected: (IndexItemModel) -> Boolean,
): ToggleableState? {
    return node.leaf?.let {
        // leaf node
        if (isSelected(it)) {
            ToggleableState.On
        } else {
            ToggleableState.Off
        }
    } ?: run {
        // internal node
        var hasOn = false
        var hasOff = false
        for (child in node.children) {
            val childState = getNodeState(child, isSelected)
            when (childState) {
                ToggleableState.On -> {
                    hasOn = true
                }

                ToggleableState.Off -> {
                    hasOff = true
                }

                ToggleableState.Indeterminate -> {
                    return ToggleableState.Indeterminate
                }

                null -> {}
            }

            if (hasOn && hasOff) {
                return ToggleableState.Indeterminate
            }
        }

        if (hasOn) {
            ToggleableState.On
        } else if (hasOff) {
            ToggleableState.Off
        } else {
            // nothing
            null
        }
    }
}

/**
 * Calls `onSelect` on all leaf nodes including and below `node` with the value of `shouldSelect`.
 */
internal fun onSelectRecursive(
    node: TreeNode,
    onSelect: (IndexItemModel, Boolean) -> Unit,
    shouldSelect: Boolean,
) {
    node.leaf?.let {
        onSelect(it, shouldSelect)
    }

    node.children.forEach {
        onSelectRecursive(it, onSelect, shouldSelect)
    }
}

internal fun LazyListScope.renderNode(
    node: TreeNode,
    isExpanded: (TreeNode) -> Boolean,
    onExpand: (TreeNode) -> Unit,
    isSelected: (IndexItemModel) -> Boolean,
    onSelect: (IndexItemModel, Boolean) -> Unit,
    nodeSizes: Map<TreeNode, FileSizeModel>,
    keyPath: String = "",
    indent: Int = 0,
) {
    val selectedState = getNodeState(node, isSelected)

    val onSelectThis = node.leaf?.let {
        // toggle selected item
        { onSelect(it, !isSelected(it)) }
    } ?: run {
        {
            // set children based on current state
            when (selectedState) {
                ToggleableState.On, ToggleableState.Indeterminate -> {
                    onSelectRecursive(node, onSelect, false)
                }

                ToggleableState.Off -> {
                    onSelectRecursive(node, onSelect, true)
                }

                null -> {}
            }
        }
    }

    item(key = "$keyPath/${node.part}") {
        TreeRow(
            node,
            isExpanded = isExpanded(node),
            onExpand = { onExpand(node) },
            selectedState = selectedState,
            onSelect = onSelectThis,
            fileSize = nodeSizes.getOrElse(node, defaultValue = { FileSizeModel.Unknown }),
            indent = indent,
        )
    }

    if (isExpanded(node)) {
        node.children.forEach { child ->
            renderNode(
                node = child,
                isExpanded = isExpanded,
                onExpand = onExpand,
                indent = indent + 1,
                isSelected = isSelected,
                onSelect = onSelect,
                nodeSizes = nodeSizes,
                keyPath = "$keyPath/${node.part}"
            )
        }
    }
}

@Composable
internal fun TreeRow(
    node: TreeNode,
    isExpanded: Boolean,
    onExpand: () -> Unit,
    selectedState: ToggleableState?,
    onSelect: () -> Unit,
    fileSize: FileSizeModel,
    indent: Int,
) {
    val degrees by animateFloatAsState(if (isExpanded) 90f else 0f)

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(56.dp)
            .clickable(onClick = {
                if (node.children.isEmpty()) {
                    onSelect()
                } else {
                    onExpand()
                }
            }),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Box(modifier = Modifier.width((indent * 24).dp))

        TriStateCheckbox(
            state = selectedState ?: ToggleableState.Off,
            enabled = selectedState != null,
            onClick = onSelect
        )

        Row(
            modifier = Modifier
                .fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                "${node.part}",
                style = MaterialTheme.typography.bodyLarge
            )

            Box(modifier = Modifier.weight(1f))

            node.leaf?.let { leaf ->
                // Text("${leaf.path}", modifier = Modifier.padding(end = 16.dp))
            } ?: run {
                val sizeMB = fileSize.value().toFloat() / 1_000_000f
                Text(
                    "${
                        if (fileSize !is FileSizeModel.Actual) {
                            "~"
                        } else {
                            ""
                        }
                    }${formatFloat(sizeMB, 1)} MB",
                    style = MaterialTheme.typography.labelLarge
                )

                Image(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = "Expand icon",
                    modifier = Modifier.padding(horizontal = 8.dp).rotate(degrees),
                    colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface),
                )
            }
        }
    }
    HorizontalDivider(thickness = 1.dp)
}

internal data class TreeNode(
    val part: String,
    val children: MutableList<TreeNode> = mutableListOf(),
    val leaf: IndexItemModel? = null,
)

internal fun countIndexFolders(index: List<IndexItemModel>): Int {
    val seen = mutableSetOf<String>()

    for (item in index) {
        // split by / and drop last part
        val path = item.path.removePrefix("/")
        val parts = path.split('/')
        val pathParts = parts.dropLast(1)

        // count unique
        val key = pathParts.joinToString("/")
        seen.add(key)
    }

    return seen.size
}

fun FileSizeModel.value(): ULong {
    return when (this) {
        is FileSizeModel.Actual -> v1
        is FileSizeModel.Estimated -> v1
        is FileSizeModel.Unknown -> 0uL
    }
}

@Composable
fun PreTransferScreenSandbox() {
    PreTransferScreen(
        onShowNodeStatus = {},

        clientModel = mockClientModel(),
        onDownloadAll = {},
        onCancel = {}
    )
}
