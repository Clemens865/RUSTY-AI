import { Volume2, Mic, Languages, Zap } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { ThemeToggle } from "./ThemeToggle";

export function Settings() {
  return (
    <div className="p-6 max-w-2xl mx-auto animate-fade-in">
      <h2 className="text-2xl font-bold mb-6 text-foreground">Settings</h2>
      
      <div className="space-y-6">
        {/* Theme Settings */}
        <Card className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-gradient-primary flex items-center justify-center">
                <Zap className="h-4 w-4 text-primary-foreground" />
              </div>
              Appearance
            </CardTitle>
            <CardDescription>
              Customize the look and feel of your voice chat interface
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center justify-between">
              <Label htmlFor="theme-toggle" className="text-sm font-medium">
                Dark Mode
              </Label>
              <ThemeToggle />
            </div>
          </CardContent>
        </Card>

        {/* Audio Settings */}
        <Card className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-gradient-secondary flex items-center justify-center">
                <Volume2 className="h-4 w-4 text-secondary-foreground" />
              </div>
              Audio Settings
            </CardTitle>
            <CardDescription>
              Adjust volume and microphone sensitivity
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-2">
              <Label className="text-sm font-medium">Output Volume</Label>
              <Slider
                defaultValue={[75]}
                max={100}
                step={1}
                className="w-full"
              />
            </div>
            
            <div className="space-y-2">
              <Label className="text-sm font-medium">Microphone Sensitivity</Label>
              <Slider
                defaultValue={[60]}
                max={100}
                step={1}
                className="w-full"
              />
            </div>
            
            <div className="flex items-center justify-between">
              <Label htmlFor="noise-reduction" className="text-sm font-medium">
                Noise Reduction
              </Label>
              <Switch id="noise-reduction" defaultChecked />
            </div>
          </CardContent>
        </Card>

        {/* Voice Settings */}
        <Card className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-gradient-primary flex items-center justify-center">
                <Mic className="h-4 w-4 text-primary-foreground" />
              </div>
              Voice Settings
            </CardTitle>
            <CardDescription>
              Configure voice recognition and response preferences
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-2">
              <Label className="text-sm font-medium">Voice Model</Label>
              <Select defaultValue="natural">
                <SelectTrigger>
                  <SelectValue placeholder="Select voice model" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="natural">Natural Voice</SelectItem>
                  <SelectItem value="professional">Professional</SelectItem>
                  <SelectItem value="casual">Casual</SelectItem>
                  <SelectItem value="energetic">Energetic</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="space-y-2">
              <Label className="text-sm font-medium">Speaking Speed</Label>
              <Slider
                defaultValue={[50]}
                max={100}
                step={1}
                className="w-full"
              />
            </div>
            
            <div className="flex items-center justify-between">
              <Label htmlFor="auto-response" className="text-sm font-medium">
                Auto Response
              </Label>
              <Switch id="auto-response" defaultChecked />
            </div>
          </CardContent>
        </Card>

        {/* Language Settings */}
        <Card className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-lg bg-accent flex items-center justify-center">
                <Languages className="h-4 w-4 text-accent-foreground" />
              </div>
              Language & Region
            </CardTitle>
            <CardDescription>
              Set your preferred language and regional settings
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-2">
              <Label className="text-sm font-medium">Language</Label>
              <Select defaultValue="en">
                <SelectTrigger>
                  <SelectValue placeholder="Select language" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="en">English</SelectItem>
                  <SelectItem value="es">Spanish</SelectItem>
                  <SelectItem value="fr">French</SelectItem>
                  <SelectItem value="de">German</SelectItem>
                  <SelectItem value="ja">Japanese</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="space-y-2">
              <Label className="text-sm font-medium">Region</Label>
              <Select defaultValue="us">
                <SelectTrigger>
                  <SelectValue placeholder="Select region" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="us">United States</SelectItem>
                  <SelectItem value="uk">United Kingdom</SelectItem>
                  <SelectItem value="ca">Canada</SelectItem>
                  <SelectItem value="au">Australia</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}