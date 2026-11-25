using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Threading;
using System.Web.Script.Serialization;
using PixelFormats = System.Windows.Media.PixelFormats;
using System.Windows.Media.Imaging;
using CodeWalker.GameFiles;
using CodeWalker.Utils;
using SharpDX;
using System.Reflection;

[assembly: AssemblyTitle("AnyRoblox FiveM exporter")]
[assembly: AssemblyDescription("Exports GTA V Legacy YDR, YDD and YFT models to OBJ for AnyRoblox")]
[assembly: AssemblyProduct("AnyRoblox")]
[assembly: AssemblyCompany("niskil377-dotcom")]
[assembly: AssemblyCopyright("See LICENSE")]
[assembly: AssemblyVersion("0.1.3.0")]
[assembly: AssemblyFileVersion("0.1.3.0")]

namespace FiveMExport {
    class Converter {
        static readonly StringComparer Names=StringComparer.OrdinalIgnoreCase;
        static bool Supported(string file) {
            return new[]{".ydr",".ydd",".yft"}.Contains(Path.GetExtension(file),Names);
        }
        static IEnumerable<string> Models(string source) {
            if(File.Exists(source)){
                if(!Supported(source))throw new InvalidDataException("Select a YDR, YDD or YFT file.");
                yield return source;
                yield break;
            }
            var pending=new Stack<string>();
            pending.Push(source);
            while(pending.Count>0){
                string dir=pending.Pop();
                foreach(string p in Directory.EnumerateFileSystemEntries(dir)){
                    var attr=File.GetAttributes(p);
                    if((attr&FileAttributes.ReparsePoint)!=0)continue;
                    if((attr&FileAttributes.Directory)!=0)pending.Push(p);
                    else if(Supported(p))yield return p;
                }
            }
        }
        static byte[] Read(string file,int version){
            using(var f=File.OpenRead(file)){
                var r=new BinaryReader(f);
                if(f.Length<16 || r.ReadUInt32()!=0x37435352)
                    throw new InvalidDataException("Unreadable or protected RSC7 resource.");
                if(r.ReadUInt32()!=version)throw new InvalidDataException("Unsupported RSC7 version. Select a GTA V Legacy model.");
                if(f.Length>512L*1024*1024)throw new InvalidDataException("File exceeds 512 MiB.");
                f.Position=0;
                return r.ReadBytes((int)f.Length);
            }
        }
        static IEnumerable<DrawableBase> Drawables(string file){
            switch(Path.GetExtension(file).ToLowerInvariant()){
                case ".ydr":
                    var ydr=new YdrFile();ydr.Load(Read(file,165));yield return ydr.Drawable;break;
                case ".ydd":
                    var ydd=new YddFile();ydd.Load(Read(file,165));
                    if(ydd.Drawables==null || ydd.Drawables.Length==0)throw new InvalidDataException("Empty drawable dictionary.");
                    foreach(var drawable in ydd.Drawables)yield return drawable;
                    break;
                case ".yft":
                    var yft=new YftFile();yft.Load(Read(file,162));
                    if(yft.Fragment==null)throw new InvalidDataException("Unreadable fragment.");
                    yield return yft.Fragment.Drawable;
                    break;
            }
        }
        static string Num(float value){
            if(float.IsNaN(value)||float.IsInfinity(value))throw new InvalidDataException("Non-finite coordinate.");
            return value.ToString("R",CultureInfo.InvariantCulture);
        }
        static void Vertex(StreamWriter w,string kind,Vector3 v){
            w.WriteLine("{0} {1} {2} {3}",kind,Num(v.X),Num(v.Z),Num(-v.Y));
        }
        static bool Has(VertexData v,VertexSemantics semantic){
            return (v.Info.Flags & (1u<<(int)semantic))!=0;
        }
        static void Geometry(StreamWriter obj,DrawableGeometry g,Matrix bind,ref int pos,ref int uv,ref int norm){
            VertexData v=g.VertexData;
            if(v==null || v.Info==null || v.VertexBytes==null || v.VertexCount<=0 ||
               (long)v.VertexCount*v.Info.Stride>v.VertexBytes.Length || !Has(v,VertexSemantics.Position) ||
               v.Info.GetComponentType(0)!=VertexComponentType.Float3)
                throw new InvalidDataException("Unsupported vertex layout.");
            int n=(int)VertexSemantics.Normal, t=(int)VertexSemantics.TexCoord0;
            bool normals=Has(v,VertexSemantics.Normal), tex=Has(v,VertexSemantics.TexCoord0);
            var nt=v.Info.GetComponentType(n);var tt=v.Info.GetComponentType(t);
            if(normals && nt!=VertexComponentType.Float3 && nt!=VertexComponentType.RGBA8SNorm)
                throw new InvalidDataException("Unsupported normal encoding.");
            if(tex && tt!=VertexComponentType.Float2 && tt!=VertexComponentType.Half2)
                throw new InvalidDataException("Unsupported UV encoding.");
            var normalMatrix=Matrix.Transpose(Matrix.Invert(bind));
            for(int i=0;i<v.VertexCount;i++){
                Vertex(obj,"v",Vector3.TransformCoordinate(v.GetVector3(i,0),bind));
                if(normals){
                    Vector3 direction;
                    if(nt==VertexComponentType.Float3)direction=v.GetVector3(i,n);
                    else {var sn=v.GetRGBA8SNorm(i,n);direction=new Vector3(sn.X,sn.Y,sn.Z);}
                    Vertex(obj,"vn",Vector3.Normalize(Vector3.TransformNormal(direction,normalMatrix)));
                }
                if(tex){
                    Vector2 st;
                    if(tt==VertexComponentType.Float2)st=v.GetVector2(i,t);
                    else {var half=v.GetHalf2(i,t);st=new Vector2(half.X,half.Y);}
                    obj.WriteLine("vt {0} {1}",Num(st.X),Num(1-st.Y));
                }
            }
            var indices=g.IndexBuffer==null?null:g.IndexBuffer.Indices;
            if(indices==null || indices.Length==0 || indices.Length%3!=0)throw new InvalidDataException("Invalid triangle indices.");
            for(int i=0;i<indices.Length;i++){
                int index=indices[i];
                if(index>=v.VertexCount)throw new InvalidDataException("Vertex index out of range.");
                if(i%3==0)obj.Write("f");
                obj.Write(" "+(pos+index));
                if(tex||normals)obj.Write("/");
                if(tex)obj.Write(uv+index);
                if(normals)obj.Write("/"+(norm+index));
                if(i%3==2)obj.WriteLine();
            }
            pos+=v.VertexCount;if(tex)uv+=v.VertexCount;if(normals)norm+=v.VertexCount;
        }
        static TextureBase Diffuse(ShaderFX shader){
            var block=shader==null?null:shader.ParametersList;
            if(block==null || block.Parameters==null)return null;
            for(int i=0;i<block.Parameters.Length;i++)
                if(Names.Equals(block.Hashes[i].ToString(),"DiffuseSampler"))return block.Parameters[i].Data as TextureBase;
            return null;
        }
        static Texture Resolve(TextureBase reference,TextureDictionary embedded,TextureDictionary external){
            var direct=reference as Texture;
            if(direct!=null && direct.Data!=null)return direct;
            foreach(var dict in new[]{embedded,external}){
                Texture match;
                if(dict!=null && dict.Dict!=null && dict.Dict.TryGetValue(reference.NameHash,out match))return match;
            }
            return null;
        }
        static void Material(StreamWriter mtl,DrawableGeometry g,TextureDictionary embedded,TextureDictionary external,string dir,List<string> errors){
            string name="material_"+g.ShaderID;
            mtl.WriteLine("newmtl "+name);mtl.WriteLine("Kd 0.85 0.85 0.85");
            var reference=Diffuse(g.Shader);
            if(reference==null)return;
            try{
                var image=Resolve(reference,embedded,external);
                if(image==null)throw new InvalidDataException("texture not found");
                if((long)image.Width*image.Height>64L*1024*1024)throw new InvalidDataException("texture too large");
                var pixels=DDSIO.GetPixels(image,0);
                if(pixels==null || pixels.Length!=4L*image.Width*image.Height)throw new InvalidDataException("unsupported texture format");
                var bitmap=BitmapSource.Create(image.Width,image.Height,96,96,PixelFormats.Bgra32,null,pixels,4*image.Width);
                var encoder=new PngBitmapEncoder();
                encoder.Frames.Add(BitmapFrame.Create(bitmap));
                using(var f=File.Create(Path.Combine(dir,name+".png")))encoder.Save(f);
                mtl.WriteLine("map_Kd "+name+".png");
            }catch(Exception ex){errors.Add(reference.Name+" : "+ex.Message);}
        }
        static void WriteDrawable(DrawableBase d,TextureDictionary external,string dir,List<string> errors){
            if(d==null || d.DrawableModels==null)throw new InvalidDataException("Missing drawable.");
            var lod=d.DrawableModels;
            var models=new[]{lod.High,lod.Med,lod.Low,lod.VLow}.FirstOrDefault(m=>m!=null && m.Length>0);
            if(models==null)throw new InvalidDataException("No visual LOD found.");
            Directory.CreateDirectory(dir);
            var materials=new HashSet<ushort>();
            int pos=1,uv=1,norm=1;
            string tmp=Path.Combine(dir,"model.obj.tmp");
            try{
                using(var obj=new StreamWriter(tmp))
                using(var mtl=new StreamWriter(Path.Combine(dir,"model.mtl"))){
                    obj.WriteLine("mtllib model.mtl");
                    foreach(var model in models){
                        Matrix bind=Matrix.Identity;
                        var bones=d.Skeleton==null||d.Skeleton.Bones==null?null:d.Skeleton.Bones.Items;
                        if(model.HasSkin==0 && bones!=null && model.BoneIndex<bones.Length)
                            bind=bones[model.BoneIndex].AbsTransform;
                        foreach(var g in model.Geometries){
                            if(materials.Add(g.ShaderID))Material(mtl,g,d.ShaderGroup==null?null:d.ShaderGroup.TextureDictionary,external,dir,errors);
                            obj.WriteLine("usemtl material_"+g.ShaderID);
                            Geometry(obj,g,bind,ref pos,ref uv,ref norm);
                        }
                    }
                }
                if(pos==1)throw new InvalidDataException("No geometry found.");
                File.Move(tmp,Path.Combine(dir,"model.obj"));
            }finally{if(File.Exists(tmp))File.Delete(tmp);}
        }
        [STAThread]
        public static int Main(string[] args){
            if(args.Length!=2)return 1;
            Thread.CurrentThread.CurrentCulture=CultureInfo.InvariantCulture;
            string source=Path.GetFullPath(args[0]),dest=Path.GetFullPath(args[1]);
            var errors=new List<string>();
            var exports=new List<string>();
            try{
                if(Directory.Exists(dest)||File.Exists(dest))throw new IOException("Output already exists.");
                if(Directory.Exists(source)&&dest.StartsWith(source.TrimEnd('\\')+"\\",StringComparison.OrdinalIgnoreCase))
                    throw new IOException("Select an output folder outside the resource.");
                Directory.CreateDirectory(dest);
                int index=0;
                foreach(string file in Models(source)){
                    try{
                        TextureDictionary external=null;
                        string ytd=Path.ChangeExtension(file,".ytd");
                        if(File.Exists(ytd)){
                            try{var dict=new YtdFile();dict.Load(Read(ytd,13));external=dict.TextureDict;}
                            catch(Exception ex){errors.Add(ytd+" : "+ex.Message);}
                        }
                        foreach(var drawable in Drawables(file)){
                            string dir=Path.Combine(dest,(++index).ToString("D4")+"-"+Path.GetFileNameWithoutExtension(file));
                            try{WriteDrawable(drawable,external,dir,errors);exports.Add(Path.GetFileName(dir));}
                            catch(Exception ex){errors.Add(file+" : "+ex.Message);}
                        }
                    }catch(Exception ex){errors.Add(file+" : "+ex.Message);}
                }
                if(exports.Count==0)errors.Add("No supported FiveM models found.");
                var json=new JavaScriptSerializer{MaxJsonLength=int.MaxValue};
                File.WriteAllText(Path.Combine(dest,"report.json"),json.Serialize(new{source=source,models=exports,errors=errors}));
                return errors.Count==0?0:2;
            }catch(Exception ex){Console.Error.WriteLine(ex.Message);return 1;}
        }
    }
}

