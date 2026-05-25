package com.crush.container

import com.intellij.execution.configurations.*
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.IconLoader
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory
import javax.swing.*
import javax.swing.table.DefaultTableModel
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken

// ── Run Configuration ──────────────────────────────────────────────────

class CrushRunConfigurationType : ConfigurationTypeBase(
    "CRUSH_RUN_CONFIG", "Crush", "Crush Container Runtime",
    IconLoader.getIcon("/icons/crush.svg", javaClass)
) {
    init {
        addFactory(object : ConfigurationFactory(this) {
            override fun getId() = "crush"
            override fun createTemplateConfiguration(project: Project) =
                CrushRunConfiguration(project, this, "Crush")
        })
    }
}

class CrushRunConfiguration(project: Project, factory: ConfigurationFactory, name: String)
    : RunConfigurationBase<CrushRunConfigurationOptions>(project, factory, name) {

    override fun getOptions() = CrushRunConfigurationOptions()
    override fun getConfigurationEditor() = CrushSettingsEditor(project)
}

class CrushRunConfigurationOptions : RunConfigurationOptions() {
    var image by string("", "Container Image")
    var ports by string("", "Port Mappings")
    var memory by string("512", "Memory Limit (MB)")
    var detach by boolean(false, "Run Detached")
}

class CrushSettingsEditor(project: Project) : SettingsEditor<CrushRunConfiguration>() {
    private val panel = JPanel().apply {
        layout = BoxLayout(this, BoxLayout.Y_AXIS)
        add(JLabel("Image:"))
        add(JTextField(30))
        add(JLabel("Ports (e.g. 8080:80):"))
        add(JTextField(30))
        add(JLabel("Memory (MB):"))
        add(JTextField("512", 10))
    }
    override fun resetEditorFrom(config: CrushRunConfiguration) {}
    override fun applyEditorTo(config: CrushRunConfiguration) {}
    override fun createComponent() = panel
}

// ── Container Tool Window ──────────────────────────────────────────────

class CrushToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = JPanel(BorderLayout())
        val columnNames = arrayOf("Container ID", "Image", "Status", "Ports")
        val tableModel = DefaultTableModel(columnNames, 0)
        val table = JTable(tableModel)
        panel.add(JScrollPane(table), BorderLayout.CENTER)

        val refreshBtn = JButton("Refresh").apply {
            addActionListener {
                try {
                    val proc = Runtime.getRuntime().exec(arrayOf(
                        "crush", "ps", "--format", "json"))
                    val json = proc.inputStream.readBytes().decodeToString()
                    proc.waitFor()

                    val containers: List<Map<String, Any?>> = Gson().fromJson(
                        json, object : TypeToken<List<Map<String, Any?>>>() {}.type
                    ) ?: emptyList()
                    tableModel.rowCount = 0
                    containers.forEach { map ->
                        tableModel.addRow(arrayOf(
                            (map["Id"] as? String)?.take(12) ?: "?",
                            (map["Image"] as? String) ?: "?",
                            (map["State"] as? Map<*, *>)?.get("Status") ?: map["Status"] ?: "?",
                            ""
                        ))
                    }
                } catch (e: Exception) {
                    tableModel.rowCount = 0
                    tableModel.addRow(arrayOf("Error", e.message ?: "connection failed", "", ""))
                }
            }
        }
        panel.add(refreshBtn, BorderLayout.SOUTH)

        // Initial load
        refreshBtn.doClick()

        val content = ContentFactory.getInstance().createContent(panel, "Containers", false)
        toolWindow.contentManager.addContent(content)
    }
}
