import { Clock, MessageSquare, Trash2, Play } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface ChatSession {
  id: string;
  title: string;
  date: string;
  duration: string;
  messageCount: number;
  preview: string;
  status: "completed" | "ongoing" | "failed";
}

const mockSessions: ChatSession[] = [
  {
    id: "1",
    title: "Morning Planning Session",
    date: "2 hours ago",
    duration: "8m 32s",
    messageCount: 12,
    preview: "Discussed project roadmap and upcoming deadlines...",
    status: "completed"
  },
  {
    id: "2",
    title: "Quick Question",
    date: "Yesterday",
    duration: "2m 15s",
    messageCount: 4,
    preview: "Asked about weather forecast and travel recommendations...",
    status: "completed"
  },
  {
    id: "3",
    title: "Learning Session",
    date: "2 days ago",
    duration: "15m 47s",
    messageCount: 23,
    preview: "Deep dive into machine learning concepts and applications...",
    status: "completed"
  },
  {
    id: "4",
    title: "Brainstorming Ideas",
    date: "3 days ago",
    duration: "12m 03s",
    messageCount: 18,
    preview: "Creative session about product features and user experience...",
    status: "completed"
  }
];

export function History() {
  const getStatusColor = (status: ChatSession["status"]) => {
    switch (status) {
      case "completed":
        return "bg-success text-success-foreground";
      case "ongoing":
        return "bg-primary text-primary-foreground";
      case "failed":
        return "bg-destructive text-destructive-foreground";
      default:
        return "bg-muted text-muted-foreground";
    }
  };

  return (
    <div className="p-6 max-w-4xl mx-auto animate-fade-in">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-foreground">Chat History</h2>
        <Button variant="outline" size="sm" className="hover:shadow-md transition-all duration-300">
          <Trash2 className="h-4 w-4 mr-2" />
          Clear All
        </Button>
      </div>
      
      <div className="space-y-4">
        {mockSessions.map((session, index) => (
          <Card 
            key={session.id} 
            className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300 animate-slide-up"
            style={{ animationDelay: `${index * 0.1}s` }}
          >
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between">
                <div className="space-y-1">
                  <CardTitle className="text-lg flex items-center gap-2">
                    <MessageSquare className="h-5 w-5 text-primary" />
                    {session.title}
                  </CardTitle>
                  <CardDescription className="flex items-center gap-4 text-sm">
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {session.date}
                    </span>
                    <span>Duration: {session.duration}</span>
                    <span>{session.messageCount} messages</span>
                  </CardDescription>
                </div>
                <Badge className={getStatusColor(session.status)}>
                  {session.status}
                </Badge>
              </div>
            </CardHeader>
            
            <CardContent className="pt-0">
              <p className="text-sm text-muted-foreground mb-4 line-clamp-2">
                {session.preview}
              </p>
              
              <div className="flex gap-2">
                <Button variant="outline" size="sm" className="hover:shadow-md transition-all duration-300">
                  <Play className="h-3 w-3 mr-2" />
                  Replay
                </Button>
                <Button variant="ghost" size="sm" className="hover:bg-destructive/10 hover:text-destructive">
                  <Trash2 className="h-3 w-3 mr-2" />
                  Delete
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      
      {mockSessions.length === 0 && (
        <div className="text-center py-12">
          <MessageSquare className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
          <h3 className="text-lg font-medium text-foreground mb-2">No chat history yet</h3>
          <p className="text-muted-foreground">
            Start a voice conversation to see your chat history here
          </p>
        </div>
      )}
    </div>
  );
}