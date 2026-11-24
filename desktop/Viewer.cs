using System;
using System.IO;
using System.Collections.Generic;
using System.Globalization;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Media.Media3D;
using System.Windows.Input;

namespace ArmaStudio {
    public class OrbitView : Grid {
        readonly Viewport3D view = new Viewport3D();
        readonly PerspectiveCamera camera = new PerspectiveCamera();
        readonly ModelVisual3D subject = new ModelVisual3D();
        Point previous;
        Point3D pivot;
        Rect3D bounds;
        double yaw, pitch, distance, radius;
        public OrbitView() {
            Background = new SolidColorBrush(Color.FromRgb(39,45,51));
            ClipToBounds = true;
            Children.Add(view);
            view.Camera = camera;
            camera.FieldOfView = 40;
            var lights = new Model3DGroup();
            lights.Children.Add(new AmbientLight(Color.FromRgb(155,155,155)));
            lights.Children.Add(new DirectionalLight(Colors.White,new Vector3D(-1,-1,-2)));
            lights.Children.Add(new DirectionalLight(Color.FromRgb(125,138,148),new Vector3D(1,0,1)));
            view.Children.Add(new ModelVisual3D { Content=lights });
            view.Children.Add(subject);
            MouseWheel += (s,e)=>{ Zoom(e.Delta); e.Handled=true; };
            MouseDown += (s,e)=>{
                if(e.ChangedButton!=MouseButton.Middle)return;
                previous=e.GetPosition(this);
                CaptureMouse(); e.Handled=true;
            };
            MouseUp += (s,e)=>{if(e.ChangedButton==MouseButton.Middle)ReleaseMouseCapture();};
            MouseMove += (s,e)=>{
                if(!IsMouseCaptured || e.MiddleButton!=MouseButtonState.Pressed)return;
                Point now=e.GetPosition(this);
                Drag(now.X-previous.X,now.Y-previous.Y,(Keyboard.Modifiers & ModifierKeys.Shift)!=0);
                previous=now;
            };
        }
        public void ShowModel(Model3D model) {
            subject.Content=model;
            bounds=model.Bounds;
            Frame();
        }
        public void Clear() {
            subject.Content=null; bounds=Rect3D.Empty; radius=0;
        }
        public void Frame() {
            if(bounds.IsEmpty)return;
            pivot=new Point3D(bounds.X+bounds.SizeX/2,bounds.Y+bounds.SizeY/2,bounds.Z+bounds.SizeZ/2);
            radius=Math.Max(1e-8,new Vector3D(bounds.SizeX,bounds.SizeY,bounds.SizeZ).Length/2);
            double aspect=ActualWidth/Math.Max(1,ActualHeight);
            yaw=0.32; pitch=0.22;
            if(bounds.SizeY<Math.Min(bounds.SizeX,bounds.SizeZ)*0.1)pitch=1.22;
            if(bounds.SizeX<Math.Min(bounds.SizeY,bounds.SizeZ)*0.1)yaw=1.3;
            var direction=Direction();
            var right=Vector3D.CrossProduct(new Vector3D(0,1,0),direction); right.Normalize();
            var up=Vector3D.CrossProduct(direction,right);
            double tangent=Math.Tan(camera.FieldOfView*Math.PI/360);
            distance=radius;
            for(int i=0;i<8;i++) {
                var corner=new Vector3D(((i&1)==0?-0.5:0.5)*bounds.SizeX,
                    ((i&2)==0?-0.5:0.5)*bounds.SizeY,((i&4)==0?-0.5:0.5)*bounds.SizeZ);
                double extent=Math.Max(Math.Abs(Vector3D.DotProduct(corner,right)),
                    Math.Abs(Vector3D.DotProduct(corner,up))*Math.Max(0.1,aspect));
                distance=Math.Max(distance,Vector3D.DotProduct(corner,direction)+1.35*extent/tangent);
            }
            UpdateCamera();
        }
        public void Zoom(double delta) {
            if(radius<=0)return;
            distance=Math.Max(radius*0.025,Math.Min(radius*150,distance*Math.Pow(0.84,delta/120)));
            UpdateCamera();
        }
        public void Drag(double dx,double dy,bool pan) {
            if(radius<=0)return;
            if(pan) {
                var forward=camera.LookDirection; forward.Normalize();
                var right=Vector3D.CrossProduct(forward,new Vector3D(0,1,0));right.Normalize();
                var up=Vector3D.CrossProduct(right,forward);
                double scale=2*distance*Math.Tan(camera.FieldOfView*Math.PI/360)/Math.Max(1,ActualWidth);
                pivot+=scale*(-dx*right+dy*up);
            } else {
                yaw-=dx*0.009;
                pitch=Math.Max(-1.55,Math.Min(1.55,pitch+dy*0.009));
            }
            UpdateCamera();
        }
        Vector3D Direction() {
            return new Vector3D(Math.Sin(yaw)*Math.Cos(pitch),Math.Sin(pitch),Math.Cos(yaw)*Math.Cos(pitch));
        }
        void UpdateCamera() {
            var offset=Direction();
            camera.Position=pivot+distance*offset;
            camera.LookDirection=-offset;
            camera.UpDirection=new Vector3D(0,1,0);
            camera.NearPlaneDistance=Math.Max(radius*0.0001,distance-radius*3);
            camera.FarPlaneDistance=distance+radius*1000;
        }
    }

    public class LoadedModel {
        public Model3DGroup Group = new Model3DGroup();
        public int Vertices, Faces;
        public string Notice="";
    }
    public static class ObjReader {
        static double Number(string s) {
            double n=double.Parse(s,CultureInfo.InvariantCulture);
            if(double.IsNaN(n)||double.IsInfinity(n))throw new InvalidDataException("Coordonnée invalide.");
            return n;
        }
        static int Index(string s,int count) {
            int i=int.Parse(s,CultureInfo.InvariantCulture);
            i=i<0?count+i:i-1;
            if(i<0||i>=count)throw new InvalidDataException("Indice OBJ hors limites.");
            return i;
        }
        static string Local(string root,string file) {
            string path=Path.GetFullPath(Path.Combine(root,file));
            if(!path.StartsWith(Path.GetFullPath(root).TrimEnd('\\')+"\\",StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Ressource hors du dossier du modèle.");
            return path;
        }
        static Material Surface(Brush brush) { return new DiffuseMaterial(brush); }
        static void ReadMaterials(string path,Dictionary<string,Material> materials,LoadedModel result) {
            if(!File.Exists(path)){result.Notice="Fichier MTL absent.";return;}
            string current="";
            foreach(string raw in File.ReadLines(path)) {
                string line=raw.Trim();
                if(line.StartsWith("newmtl ")) current=line.Substring(7).Trim();
                if(line.StartsWith("Kd ")) {
                    string[] p=line.Split((char[])null,StringSplitOptions.RemoveEmptyEntries);
                    materials[current]=Surface(new SolidColorBrush(Color.FromScRgb(1,(float)Number(p[1]),(float)Number(p[2]),(float)Number(p[3]))));
                }
                if(line.StartsWith("map_Kd ")) {
                    string image=Local(Path.GetDirectoryName(path),line.Substring(7).Trim());
                    if(!File.Exists(image)){result.Notice="Texture absente.";continue;}
                    var bitmap=new BitmapImage();
                    using(var file=File.OpenRead(image)){
                        bitmap.BeginInit();bitmap.CacheOption=BitmapCacheOption.OnLoad;bitmap.StreamSource=file;bitmap.EndInit();
                    }
                    bitmap.Freeze();
                    materials[current]=Surface(new ImageBrush(bitmap){TileMode=TileMode.Tile,ViewportUnits=BrushMappingMode.Absolute,Viewport=new Rect(0,0,1,1)});
                }
            }
        }
        public static LoadedModel Read(string path) {
            var result=new LoadedModel();
            var points=new List<Point3D>();
            var normals=new List<Vector3D>();
            var uv=new List<Point>();
            var materials=new Dictionary<string,Material>();
            var meshes=new Dictionary<string,MeshGeometry3D>();
            string current="";
            foreach(string raw in File.ReadLines(path)) {
                string line=raw.Split('#')[0].Trim();
                if(line.Length==0)continue;
                string[] p=line.Split((char[])null,StringSplitOptions.RemoveEmptyEntries);
                switch(p[0]) {
                case "v": points.Add(new Point3D(Number(p[1]),Number(p[2]),Number(p[3])));break;
                case "vn": normals.Add(new Vector3D(Number(p[1]),Number(p[2]),Number(p[3])));break;
                case "vt": uv.Add(new Point(Number(p[1]),1-Number(p[2])));break;
                case "mtllib": ReadMaterials(Local(Path.GetDirectoryName(path),line.Substring(6).Trim()),materials,result);break;
                case "usemtl": current=line.Substring(6).Trim();break;
                case "f":
                    if(p.Length<4)throw new InvalidDataException("Face incomplète.");
                    MeshGeometry3D mesh;
                    if(!meshes.TryGetValue(current,out mesh))meshes[current]=mesh=new MeshGeometry3D();
                    int start=mesh.Positions.Count;
                    for(int i=1;i<p.Length;i++) {
                        var c=p[i].Split('/');
                        mesh.Positions.Add(points[Index(c[0],points.Count)]);
                        mesh.TextureCoordinates.Add(c.Length>1&&c[1]!=""?uv[Index(c[1],uv.Count)]:new Point());
                        if(c.Length>2&&c[2]!="")mesh.Normals.Add(normals[Index(c[2],normals.Count)]);
                        if(i>=3){mesh.TriangleIndices.Add(start);mesh.TriangleIndices.Add(start+i-2);mesh.TriangleIndices.Add(start+i-1);}
                    }
                    result.Faces++;break;
                }
            }
            foreach(var part in meshes) {
                Material mat;
                if(!materials.TryGetValue(part.Key,out mat))mat=Surface(Brushes.LightGray);
                if(part.Value.Normals.Count!=part.Value.Positions.Count)part.Value.Normals.Clear();
                result.Group.Children.Add(new GeometryModel3D(part.Value,mat){BackMaterial=mat});
            }
            if(result.Faces==0)throw new InvalidDataException("Aucune face à afficher.");
            result.Vertices=points.Count;
            result.Group.Freeze();
            return result;
        }
    }
}
