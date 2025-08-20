import { useState, useRef, useEffect } from "react";
import { Upload, Search, FileText, X, Tag, Brain } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";

interface SearchResult {
  id: string;
  title: string;
  content: string;
  score: number;
  chunk_index: number;
  source: string;
}

export function KnowledgeBase() {
  const [uploadTitle, setUploadTitle] = useState("");
  const [uploadContent, setUploadContent] = useState("");
  const [uploadTags, setUploadTags] = useState("");
  const [uploadStatus, setUploadStatus] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [stats, setStats] = useState<any>(null);
  
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Fetch knowledge base statistics
  const fetchStats = async () => {
    try {
      const response = await fetch('/api/v1/knowledge/stats');
      if (response.ok) {
        const data = await response.json();
        setStats(data);
      }
    } catch (error) {
      console.error("Failed to fetch stats:", error);
    }
  };

  // Upload document
  const handleUpload = async () => {
    if (!uploadTitle || !uploadContent) {
      setUploadStatus("Please provide both title and content");
      return;
    }

    setIsUploading(true);
    setUploadStatus(null);

    const formData = new FormData();
    formData.append("title", uploadTitle);
    formData.append("content", uploadContent);
    formData.append("source", "manual");
    formData.append("tags", uploadTags);

    try {
      const response = await fetch('/api/v1/knowledge/upload', {
        method: 'POST',
        body: formData,
      });

      if (response.ok) {
        const result = await response.json();
        setUploadStatus(`✅ Document uploaded successfully! Created ${result.chunks_created} chunks.`);
        setUploadTitle("");
        setUploadContent("");
        setUploadTags("");
        fetchStats();
      } else {
        setUploadStatus("❌ Failed to upload document");
      }
    } catch (error) {
      console.error("Upload error:", error);
      setUploadStatus("❌ Error uploading document");
    } finally {
      setIsUploading(false);
    }
  };

  // Handle file upload
  const handleFileUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setIsUploading(true);
    setUploadStatus(null);

    const formData = new FormData();
    formData.append("file", file);
    formData.append("title", file.name);
    formData.append("source", file.name);
    formData.append("tags", uploadTags);

    // Read file content as text
    const reader = new FileReader();
    reader.onload = async (e) => {
      const content = e.target?.result as string;
      formData.append("content", content);

      try {
        const response = await fetch('/api/v1/knowledge/upload', {
          method: 'POST',
          body: formData,
        });

        if (response.ok) {
          const result = await response.json();
          setUploadStatus(`✅ File uploaded successfully! Created ${result.chunks_created} chunks.`);
          fetchStats();
        } else {
          setUploadStatus("❌ Failed to upload file");
        }
      } catch (error) {
        console.error("File upload error:", error);
        setUploadStatus("❌ Error uploading file");
      } finally {
        setIsUploading(false);
      }
    };
    reader.readAsText(file);
  };

  // Search documents
  const handleSearch = async () => {
    if (!searchQuery) return;

    setIsSearching(true);
    setSearchResults([]);

    try {
      const params = new URLSearchParams({
        query: searchQuery,
        limit: "10",
        threshold: "0.3",
      });

      const response = await fetch(`/api/v1/knowledge/search?${params}`);
      
      if (response.ok) {
        const data = await response.json();
        setSearchResults(data.documents);
      } else {
        console.error("Search failed");
      }
    } catch (error) {
      console.error("Search error:", error);
    } finally {
      setIsSearching(false);
    }
  };

  // Load stats on mount
  useEffect(() => {
    fetchStats();
  }, []);

  return (
    <div className="max-w-6xl mx-auto p-4 space-y-6">
      {/* Header with Stats */}
      <Card className="p-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold flex items-center gap-2">
              <Brain className="h-6 w-6 text-primary" />
              Knowledge Base
            </h2>
            <p className="text-muted-foreground mt-1">
              Upload documents and search your personal knowledge base
            </p>
          </div>
          {stats && (
            <div className="text-right">
              <p className="text-sm text-muted-foreground">Vectors stored</p>
              <p className="text-2xl font-bold">{stats.vectors_count || 0}</p>
            </div>
          )}
        </div>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Upload Section */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
            <Upload className="h-5 w-5" />
            Upload Document
          </h3>
          
          <div className="space-y-4">
            <div>
              <Label htmlFor="title">Title</Label>
              <Input
                id="title"
                value={uploadTitle}
                onChange={(e) => setUploadTitle(e.target.value)}
                placeholder="Document title..."
                disabled={isUploading}
              />
            </div>

            <div>
              <Label htmlFor="content">Content</Label>
              <Textarea
                id="content"
                value={uploadContent}
                onChange={(e) => setUploadContent(e.target.value)}
                placeholder="Paste your document content here..."
                rows={8}
                disabled={isUploading}
              />
            </div>

            <div>
              <Label htmlFor="tags">Tags (comma-separated)</Label>
              <Input
                id="tags"
                value={uploadTags}
                onChange={(e) => setUploadTags(e.target.value)}
                placeholder="work, personal, important..."
                disabled={isUploading}
              />
            </div>

            <div className="flex gap-2">
              <Button 
                onClick={handleUpload} 
                disabled={isUploading}
                className="flex-1"
              >
                {isUploading ? "Uploading..." : "Upload Text"}
              </Button>
              
              <Button
                onClick={() => fileInputRef.current?.click()}
                variant="outline"
                disabled={isUploading}
                className="flex-1"
              >
                <FileText className="h-4 w-4 mr-2" />
                Upload File
              </Button>
              
              <input
                ref={fileInputRef}
                type="file"
                accept=".txt,.md,.json,.csv"
                onChange={handleFileUpload}
                className="hidden"
              />
            </div>

            {uploadStatus && (
              <Alert>
                <AlertDescription>{uploadStatus}</AlertDescription>
              </Alert>
            )}
          </div>
        </Card>

        {/* Search Section */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
            <Search className="h-5 w-5" />
            Search Knowledge
          </h3>
          
          <div className="space-y-4">
            <div className="flex gap-2">
              <Input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                placeholder="Search your documents..."
                disabled={isSearching}
              />
              <Button 
                onClick={handleSearch} 
                disabled={isSearching}
              >
                {isSearching ? "Searching..." : "Search"}
              </Button>
            </div>

            {/* Search Results */}
            <div className="space-y-3 max-h-[500px] overflow-y-auto">
              {searchResults.length > 0 ? (
                searchResults.map((result, index) => (
                  <Card key={`${result.id}_${index}`} className="p-4">
                    <div className="flex items-start justify-between mb-2">
                      <h4 className="font-semibold">{result.title}</h4>
                      <Badge variant="secondary">
                        {(result.score * 100).toFixed(1)}%
                      </Badge>
                    </div>
                    <p className="text-sm text-muted-foreground line-clamp-3">
                      {result.content}
                    </p>
                    <div className="flex items-center gap-2 mt-2">
                      <span className="text-xs text-muted-foreground">
                        Chunk {result.chunk_index + 1}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        Source: {result.source}
                      </span>
                    </div>
                  </Card>
                ))
              ) : searchQuery && !isSearching ? (
                <p className="text-muted-foreground text-center py-8">
                  No results found. Try a different search term.
                </p>
              ) : (
                <p className="text-muted-foreground text-center py-8">
                  Enter a search query to find relevant documents
                </p>
              )}
            </div>
          </div>
        </Card>
      </div>

      {/* Info Card */}
      <Card className="p-6 bg-muted/50">
        <h3 className="font-semibold mb-2">How it works</h3>
        <ul className="space-y-1 text-sm text-muted-foreground">
          <li>• Upload documents to build your personal knowledge base</li>
          <li>• Documents are split into chunks and embedded using OpenAI</li>
          <li>• Vectors are stored in Qdrant for semantic search</li>
          <li>• When you chat, relevant context is automatically retrieved</li>
          <li>• The AI uses your documents to provide personalized answers</li>
        </ul>
      </Card>
    </div>
  );
}