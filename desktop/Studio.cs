using System;
using System.IO;
using System.Diagnostics;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Markup;
using System.Web.Script.Serialization;
using Microsoft.Win32;
using Forms=System.Windows.Forms;

namespace ArmaStudio {
    public class Studio {
        readonly Window window;
        readonly OrbitView orbit=new OrbitView();
        readonly ListBox models;
        readonly string local=Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),"Arma2OBJ");
        string output;
        int revision;
        List<string> roots=new List<string>();
        readonly ComboBox mods;
        string library;
        bool busy;
        bool fiveM;
        class Choice {
            public string PathName;
            public override string ToString(){return Label(PathName);}
        }
        static string Label(string file){
            return Path.GetFileName(file).Equals("model.obj",StringComparison.OrdinalIgnoreCase)?
                Path.GetFileName(Path.GetDirectoryName(file)):Path.GetFileNameWithoutExtension(file);
        }
        T Get<T>(string name) where T:class {return (T)window.FindName(name);}
        void Status(string message){Get<TextBlock>("Status").Text=message;}
        Studio(string[] args){
            using(var xaml=Resource("Main.xaml"))window=(Window)XamlReader.Load(xaml);
            Get<Grid>("Scene").Children.Insert(0,orbit);
            var solidToggle=Get<CheckBox>("SolidToggle");
            solidToggle.Checked+=(s,e)=>orbit.SetSolid(true);
            solidToggle.Unchecked+=(s,e)=>orbit.SetSolid(false);
            var gridToggle=Get<CheckBox>("GridToggle");
            gridToggle.Checked+=(s,e)=>orbit.SetGrid(true);
            gridToggle.Unchecked+=(s,e)=>orbit.SetGrid(false);
            Get<DockPanel>("Editor").IsEnabled=false;
            models=Get<ListBox>("Models");
            mods=Get<ComboBox>("Mods");
            library=ModCatalog.Text(Path.Combine(local,"mods-folder.txt")).Trim();
            Get<Button>("ChooseFiveM").Click+=async (s,e)=>await ChooseGame(true);
            Get<Button>("ChooseArma").Click+=async (s,e)=>await ChooseGame(false);
            Get<Button>("ChangeGame").Click+=(s,e)=>{
                Get<DockPanel>("Editor").IsEnabled=false;
                Get<Grid>("Home").Visibility=Visibility.Visible;
            };
            mods.SelectionChanged+=(s,e)=>Selection();
            Get<Button>("Refresh").Click+=async (s,e)=>await Refresh();
            Get<Button>("Library").Click+=async (s,e)=>{
                string chosen=PickFolder("Choose your mod library",BrowseRoot());
                if(chosen==null)return;
                library=chosen;
                try{Directory.CreateDirectory(local);File.WriteAllText(Path.Combine(local,"mods-folder.txt"),library);}
                catch(Exception ex){Status("Library selected for this session. "+ex.Message);}
                await Refresh();
            };
            Get<Button>("ConvertButton").Click+=(s,e)=>{
                var mod=mods.SelectedItem as ModSource;
                if(mod!=null)StartImport(fiveM?"fivem":"--input",mod.PathName);
            };
            models.SelectionChanged+=async (s,e)=>{
                Choice choice=models.SelectedItem as Choice;
                if(choice!=null)await Show(choice.PathName);
            };
            Get<Button>("OpenObj").Click+=(s,e)=>{
                var file=new OpenFileDialog{Filter="Wavefront model (*.obj)|*.obj",Multiselect=true};
                if(file.ShowDialog(window)==true)Populate(file.FileNames);
            };
            Get<Button>("Folder").Click+=(s,e)=>{
                string folder=PickFolder(fiveM?"Select a FiveM resource folder":"Select a mod folder",fiveM?"":BrowseRoot());
                if(folder!=null)SelectSource(folder);
            };
            Get<Button>("Archive").Click+=(s,e)=>{
                var file=new OpenFileDialog{Filter=fiveM?"FiveM models (*.ydr;*.ydd;*.yft)|*.ydr;*.ydd;*.yft":"Arma archives (*.pbo;*.zip)|*.pbo;*.zip",InitialDirectory=fiveM?"":BrowseRoot()};
                if(file.ShowDialog(window)==true)SelectSource(file.FileName);
            };
            Get<Button>("Results").Click+=(s,e)=>{if(Directory.Exists(output))Process.Start("explorer.exe",Quote(output));};
            Get<Button>("Frame").Click+=(s,e)=>orbit.Frame();
            window.PreviewKeyDown+=(s,e)=>{if(e.Key==Key.F && !(Keyboard.FocusedElement is TextBox)){orbit.Frame();e.Handled=true;}};
            window.Closing+=(s,e)=>{if(busy){e.Cancel=true;Status("Conversion is running. You can close the window when it finishes.");}};
            if(args.Length>0)window.ContentRendered+=async (s,e)=>{
                await ChooseGame(false);
                Populate(args.Select(Path.GetFullPath));
            };
        }
        async Task ChooseGame(bool fromFiveM){
            fiveM=fromFiveM;
            Get<Grid>("Home").Visibility=Visibility.Collapsed;
            Get<DockPanel>("Editor").Visibility=Visibility.Visible;
            Get<DockPanel>("Editor").IsEnabled=true;
            Get<TextBlock>("GameTitle").Text=fiveM?"FiveM → OBJ":"Arma 3 → OBJ";
            Get<TextBlock>("InputTitle").Text=fiveM?"FiveM resource":"Installed mods";
            Get<TextBlock>("Instructions").Text=fiveM?
                "Choose a GTA V Legacy model or a resource’s stream folder, then convert it to OBJ.":
                "Download the mod in Steam, refresh the list, then select the installed mod to convert.";
            Get<WrapPanel>("ArmaLibrary").Visibility=fiveM?Visibility.Collapsed:Visibility.Visible;
            Get<Button>("Archive").Content=fiveM?"YDR / YDD / YFT…":"PBO / ZIP…";
            mods.Items.Clear();
            Populate(new string[0]);
            output=null;
            Get<Button>("Results").IsEnabled=false;
            Status("Ready");
            if(fiveM)Selection();
            else await Refresh();
        }
        string BrowseRoot(){
            return Directory.Exists(library)?library:roots.FirstOrDefault(Directory.Exists)??"";
        }
        async Task Refresh(){
            var previous=mods.SelectedItem as ModSource;
            Get<StackPanel>("Controls").IsEnabled=false;
            Get<Button>("ChangeGame").IsEnabled=false;
            try{
                roots=ModCatalog.Roots(ModCatalog.Steam(),AppDomain.CurrentDomain.BaseDirectory,library);
                var found=await Task.Run(()=>ModCatalog.Scan(roots));
                mods.Items.Clear();
                foreach(var mod in found)mods.Items.Add(mod);
                if(previous!=null && (Directory.Exists(previous.PathName)||File.Exists(previous.PathName)))SelectSource(previous.PathName);
                Selection();
            }catch(Exception ex){Status("Could not read the mod folders. "+ex.Message);}
            finally{Get<StackPanel>("Controls").IsEnabled=true;Get<Button>("ChangeGame").IsEnabled=true;}
        }
        void SelectSource(string path){
            var item=mods.Items.Cast<ModSource>().FirstOrDefault(m=>string.Equals(m.PathName,path,StringComparison.OrdinalIgnoreCase));
            if(item==null){item=new ModSource(path);mods.Items.Add(item);}
            mods.SelectedItem=item;
        }
        void Selection(){
            var mod=mods.SelectedItem as ModSource;
            var hint=Get<TextBlock>("SourceHint");
            hint.Text=mod!=null?mod.PathName:fiveM?"No resource selected.":mods.Items.Count==0?"No installed mods found. Choose a library, folder or archive.":"Select an installed mod.";
            hint.ToolTip=mod!=null?mod.PathName:fiveM?null:string.Join(Environment.NewLine,roots);
            Get<Button>("ConvertButton").IsEnabled=mod!=null;
        }
        static Stream Resource(string name) {
            var stream=Assembly.GetExecutingAssembly().GetManifestResourceStream(name);
            if(stream==null)throw new FileNotFoundException("Missing resource",name);
            return stream;
        }
        async Task Show(string path) {
            int request=++revision;
            try{
                Status("Loading "+Label(path)+"…");
                LoadedModel m=await Task.Run(()=>ObjReader.Read(path));
                if(request!=revision)return;
                orbit.ShowModel(m.Group);
                Get<TextBlock>("Empty").Visibility=Visibility.Collapsed;
                Get<TextBlock>("TitleText").Text=Label(path);
                Get<TextBlock>("Stats").Text=m.Vertices+" vertices   ·   "+m.Faces+" faces";
                Status(m.Notice==""?path:m.Notice+"  "+path);
            }catch(Exception ex){if(request==revision)Status("Could not open model. "+ex.Message);}
        }
        void Populate(IEnumerable<string> paths){
            revision++;
            orbit.Clear();
            Get<TextBlock>("TitleText").Text="Scene";
            Get<TextBlock>("Stats").Text="";
            Get<TextBlock>("Empty").Visibility=Visibility.Visible;
            models.Items.Clear();
            foreach(string p in paths)models.Items.Add(new Choice{PathName=p});
            if(models.Items.Count>0)models.SelectedIndex=0;
        }
        string PickFolder(string title,string initial=""){
            using(var dialog=new Forms.FolderBrowserDialog{Description=title,SelectedPath=initial,ShowNewFolderButton=true}){
                return dialog.ShowDialog()==Forms.DialogResult.OK?dialog.SelectedPath:null;
            }
        }
        static string Quote(string s){
            // All paths below refer to files/directories, and CLI source text is a single argument.
            return "\""+System.Text.RegularExpressions.Regex.Replace(s,@"(\\*)(""|$)",m=>
                new string('\\',m.Groups[1].Length*2)+(m.Groups[2].Value=="\""?"\\\"":""))+"\"";
        }
        static void Extract(string name,string dir){
            using(var source=Resource(name))
            using(var file=new FileStream(Path.Combine(dir,name),FileMode.CreateNew,FileAccess.Write))source.CopyTo(file);
        }
        // Only reuse helpers extracted by this running instance. A new build starts with fresh binaries.
        static readonly Dictionary<string,string> ToolDirectories=new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase);
        static Studio(){
            AppDomain.CurrentDomain.ProcessExit+=(s,e)=>{
                lock(ToolDirectories){
                    foreach(string dir in ToolDirectories.Values){
                        try{Directory.Delete(dir,true);}
                        catch(IOException){}
                        catch(UnauthorizedAccessException){}
                    }
                }
            };
        }
        static string PrepareTools(string local,bool gta){
            string sessions=Path.Combine(Path.GetFullPath(local),"sessions");
            string key=Path.Combine(sessions,gta?"fivem":"arma");
            lock(ToolDirectories){
                string dir;
                if(ToolDirectories.TryGetValue(key,out dir))return dir;
                dir=Path.Combine(sessions,Guid.NewGuid().ToString("N"));
                Directory.CreateDirectory(dir);
                try{
                    string[] files=gta?new[]{"FiveMExport.exe","CodeWalker.Core.dll","SharpDX.dll","SharpDX.Mathematics.dll","CodeWalker-Notice.txt"}:new[]{"arma2obj.exe","P3DDebin.exe","hemtt.exe"};
                    foreach(string name in files)Extract(name,dir);
                    ToolDirectories.Add(key,dir);
                    return dir;
                }catch{
                    Directory.Delete(dir,true);
                    throw;
                }
            }
        }
        public static async Task<int> Convert(string local,string flag,string source,string destination) {
            bool gta=flag=="fivem";
            string tools=PrepareTools(local,gta);
            var info=new ProcessStartInfo(Path.Combine(tools,gta?"FiveMExport.exe":"arma2obj.exe")){
                WorkingDirectory=local,UseShellExecute=false,CreateNoWindow=true,
                RedirectStandardOutput=true,RedirectStandardError=true,
                Arguments=gta?Quote(source)+" "+Quote(destination):flag+" "+Quote(source)+" --output "+Quote(destination)
            };
            if(string.IsNullOrWhiteSpace(info.EnvironmentVariables["ARMA2OBJ_DEBINARIZER"]))info.EnvironmentVariables["ARMA2OBJ_DEBINARIZER"]=Path.Combine(tools,"P3DDebin.exe");
            if(string.IsNullOrWhiteSpace(info.EnvironmentVariables["ARMA2OBJ_PAA_TOOL"]))info.EnvironmentVariables["ARMA2OBJ_PAA_TOOL"]=Path.Combine(tools,"hemtt.exe");
            using(var p=Process.Start(info)){
                var stdout=p.StandardOutput.ReadToEndAsync();
                var stderr=p.StandardError.ReadToEndAsync();
                await Task.WhenAll(stdout,stderr).ConfigureAwait(false);
                p.WaitForExit();
                if(!File.Exists(Path.Combine(destination,"report.json")))throw new IOException(stderr.Result.Trim());
                return p.ExitCode;
            }
        }
        static Dictionary<string,object> Report(string path){
            return new JavaScriptSerializer{MaxJsonLength=int.MaxValue}.Deserialize<Dictionary<string,object>>(File.ReadAllText(path));
        }
        static string ErrorText(string path){
            object errors;
            if(Report(path).TryGetValue("errors",out errors)){
                var items=errors as object[];
                if(items!=null && items.Length>0)return string.Join(" · ",items.Take(2));
            }
            return "";
        }
        async void StartImport(string flag,string source){
            if(busy)return;
            if(!Directory.Exists(source)&&!File.Exists(source)){Status("Source not found. Refresh the list.");return;}
            string parent=PickFolder("Choose where to save the converted models");
            if(parent==null)return;
            string destination=Path.Combine(parent,(fiveM?"AnyRoblox-FiveM-":"AnyRoblox-Arma3-")+DateTime.Now.ToString("yyyyMMdd-HHmmss")+"-"+Guid.NewGuid().ToString("N").Substring(0,4));
            output=destination;
            busy=true;
            Get<Button>("ChangeGame").IsEnabled=false;
            Get<StackPanel>("Controls").IsEnabled=false;
            Get<Button>("OpenObj").IsEnabled=false;
            Get<Button>("Results").IsEnabled=false;
            Status("Converting…");
            try{
                int code=await Task.Run(()=>Convert(local,flag,source,destination));
                string[] objs=Directory.GetFiles(destination,"model.obj",SearchOption.AllDirectories).OrderBy(x=>x).ToArray();
                string issues=ErrorText(Path.Combine(destination,"report.json"));
                Populate(objs);
                if(code!=0){
                    string message=issues==""?"See the model reports in the output folder.":issues;
                    MessageBox.Show(window,message,"Conversion incomplete",MessageBoxButton.OK,MessageBoxImage.Information);
                    Status(message);
                }
            }catch(Exception ex){Status("Conversion stopped. "+ex.Message);}
            finally{
                busy=false;Get<StackPanel>("Controls").IsEnabled=true;Get<Button>("OpenObj").IsEnabled=true;
                Get<Button>("ChangeGame").IsEnabled=true;
                Get<Button>("Results").IsEnabled=Directory.Exists(output);
            }
        }
        [STAThread]
        public static void Main(string[] args){
            try{
                var app=new Application();
                var studio=new Studio(args);
                app.Run(studio.window);
            }catch(Exception ex){MessageBox.Show(ex.Message,"AnyRoblox");}
        }
    }
}
