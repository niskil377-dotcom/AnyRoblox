using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace ArmaStudio {
    public sealed class ModSource {
        public string PathName {get;private set;}
        public string Name {get;private set;}
        public ModSource(string path){
            PathName=Path.GetFullPath(path);
            Name=Path.GetFileName(PathName.TrimEnd(Path.DirectorySeparatorChar));
            foreach(string metadata in new[]{"meta.cpp","mod.cpp"}){
                var match=Regex.Match(ModCatalog.Text(Path.Combine(path,metadata)),@"^\s*name\s*=\s*""([^""\r\n]+)""",RegexOptions.Multiline|RegexOptions.IgnoreCase);
                if(match.Success){Name=match.Groups[1].Value;break;}
            }
        }
        public override string ToString(){return Name;}
    }
    public static class ModCatalog {
        static readonly StringComparer Comparer=StringComparer.OrdinalIgnoreCase;
        public static string Text(string file){
            try{
                if(!File.Exists(file)||new FileInfo(file).Length>1024*1024)return "";
                return File.ReadAllText(file);
            }catch(IOException){return "";}catch(UnauthorizedAccessException){return "";}
        }
        static string[] Entries(string path,bool folders,string pattern){
            try{
                if(!Directory.Exists(path))return new string[0];
                return folders?Directory.GetDirectories(path,pattern):Directory.GetFiles(path,pattern);
            }catch(IOException){return new string[0];}catch(UnauthorizedAccessException){return new string[0];}
        }
        public static List<string> Roots(string steam,string appDir,string custom){
            var roots=new List<string>();
            if(!string.IsNullOrEmpty(steam)){
                var libraries=new HashSet<string>(Comparer){steam};
                foreach(Match m in Regex.Matches(Text(Path.Combine(steam,@"steamapps\libraryfolders.vdf")),@"""path""\s*""([^""]+)""",RegexOptions.IgnoreCase))
                    libraries.Add(m.Groups[1].Value.Replace(@"\\",@"\"));
                foreach(var lib in libraries){
                    roots.Add(Path.Combine(lib,@"steamapps\workshop\content\107410"));
                    string game=Path.Combine(lib,@"steamapps\common\Arma 3");
                    roots.Add(Path.Combine(game,"!Workshop"));
                    roots.Add(Path.Combine(game,"mods"));
                }
            }
            roots.Add(Path.Combine(appDir,"mods"));
            if(!string.IsNullOrWhiteSpace(custom)&&Path.IsPathRooted(custom))roots.Add(custom);
            return roots.Distinct(Comparer).ToList();
        }
        public static string Steam(){
            return Registry.GetValue(@"HKEY_CURRENT_USER\Software\Valve\Steam","SteamPath",null) as string ??
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),"Steam");
        }
        public static List<ModSource> Scan(IEnumerable<string> roots){
            var paths=new HashSet<string>(Comparer);
            foreach(var root in roots){
                foreach(var dir in Entries(root,true,"*"))paths.Add(dir);
                foreach(var pattern in new[]{"*.pbo","*.zip"})foreach(var file in Entries(root,false,pattern))paths.Add(file);
            }
            return paths.Select(p=>new ModSource(p)).OrderBy(m=>m.Name,Comparer).ThenBy(m=>m.PathName,Comparer).ToList();
        }
    }
}
