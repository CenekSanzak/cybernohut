import React, { useRef } from "react";
import { Download, Upload, Share2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { GraphState } from "@/state/graphReducer";
import { generateShareableURL, copyToClipboard } from "@/utils/urlSharing";
import { showNotification } from "@/utils/notifications";

interface GraphControlsProps {
  state: GraphState;
  onImport: (file: File) => void;
  onClear: () => void;
}

export const GraphControls: React.FC<GraphControlsProps> = ({
  state,
  onImport,
  onClear,
}) => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleExport = () => {
    try {
      const graphData = {
        version: "1.0",
        timestamp: new Date().toISOString(),
        nodes: state.nodes,
        edges: state.edges,
        selectedInputId: state.selectedInputId,
        input: state.input,
        selectedNodeId: state.selectedNodeId,
        selectedNodeTitle: state.selectedNodeTitle,
      };

      const jsonString = JSON.stringify(graphData, null, 2);
      const blob = new Blob([jsonString], { type: "application/json" });
      const url = URL.createObjectURL(blob);

      const timestamp = new Date()
        .toISOString()
        .replace(/[:.]/g, "-")
        .slice(0, 19);
      const filename = `cybernohut-graph-${timestamp}.json`;

      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);

      showNotification("Graph exported successfully!", "success");
    } catch (error) {
      console.error("Export failed:", error);
      showNotification("Failed to export graph", "error");
    }
  };

  const handleImport = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const content = e.target?.result as string;
        const parsed = JSON.parse(content);

        // Validate structure
        if (!parsed.nodes || !parsed.edges) {
          throw new Error("Invalid graph file format");
        }

        onImport(file);
        showNotification("Graph imported successfully!", "success");
      } catch (error) {
        console.error("Import failed:", error);
        showNotification("Invalid graph file", "error");
      }
    };

    reader.readAsText(file);
    // Reset input
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const handleShare = async () => {
    try {
      const shareableURL = generateShareableURL(state);
      const success = await copyToClipboard(shareableURL);

      if (success) {
        showNotification("Link copied to clipboard!", "success");
        // Update browser URL
        window.history.pushState({}, "", shareableURL);
      } else {
        showNotification("Failed to copy link", "error");
      }
    } catch (error) {
      console.error("Share failed:", error);
      showNotification("Failed to create shareable link", "error");
    }
  };

  const handleClear = () => {
    if (
      window.confirm(
        "Are you sure you want to clear the current graph? This action cannot be undone."
      )
    ) {
      onClear();
      showNotification("Graph cleared", "info");
    }
  };

  return (
    <div className="flex items-center gap-2 p-2 bg-white border-b border-gray-200">
      <Button
        variant="outline"
        size="sm"
        onClick={handleExport}
        className="flex items-center gap-2"
      >
        <Download size={16} />
        Export
      </Button>

      <Button
        variant="outline"
        size="sm"
        onClick={() => fileInputRef.current?.click()}
        className="flex items-center gap-2"
      >
        <Upload size={16} />
        Import
      </Button>

      <input
        ref={fileInputRef}
        type="file"
        accept=".json"
        onChange={handleImport}
        className="hidden"
        aria-label="Import graph file"
      />

      <Button
        variant="outline"
        size="sm"
        onClick={handleShare}
        className="flex items-center gap-2"
      >
        <Share2 size={16} />
        Share
      </Button>

      <Button
        variant="outline"
        size="sm"
        onClick={handleClear}
        className="flex items-center gap-2 text-red-600 hover:text-red-700 hover:bg-red-50"
      >
        <Trash2 size={16} />
        Clear
      </Button>
    </div>
  );
};
