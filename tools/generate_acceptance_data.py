"""Generate second-batch acceptance CSVs and independent NumPy/SciPy expectations.
Run: python tools/generate_acceptance_data.py. No random numbers are used.
Requires the pinned NumPy/SciPy versions in tools/reference/requirements-reference.txt.
Writes only docs/acceptance/data; existing scientific references are read-only.
"""
from pathlib import Path
from itertools import zip_longest
import csv
import json
import numpy as np
import scipy
from scipy import optimize, signal, stats

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/acceptance/data'
REF = ROOT / 'crates/scicanvas-core/tests/reference'


def write_csv(name, columns):
    OUT.mkdir(parents=True, exist_ok=True)
    with (OUT / (name + '.csv')).open('w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f, lineterminator='\n')
        writer.writerow(columns)
        for row in zip_longest(*columns.values(), fillvalue=None):
            writer.writerow('' if v is None or isinstance(v, (float, np.floating)) and not np.isfinite(v)
                            else format(float(v), '.17g') for v in row)


def cases(name):
    return json.loads((REF / (name + '.json')).read_text(encoding='utf-8'))['cases']


def main():
    expected = {'generator': 'tools/generate_acceptance_data.py',
                'versions': {'NumPy': np.__version__, 'SciPy': scipy.__version__}, 'cases': {}}
    e = expected['cases']
    a, h = np.array([1., 2., 3.]), np.array([1., 1.])
    write_csv('convolution', {'signal': a, 'kernel': h})
    e['F206'] = {mode: np.convolve(a, h, mode).tolist() for mode in ['full', 'same', 'valid']}
    # [1,1] has a zero at the Nyquist frequency. Use an invertible second kernel for pure FFT division.
    h2 = np.array([1., .5]); full = np.convolve(a, h2)
    write_csv('deconvolution', {'full_signal': full, 'kernel': h2})
    q, remainder = signal.deconvolve(full, h2)
    e['F206'].update(deconvolution=q.tolist() + [0.], remainder=remainder.tolist(), epsilon=0,
                    explanation='The extra output row is zero padding; the first three rows recover [1,2,3].')
    x = np.arange(5.); y = 2*x+1; target = np.array([2.5, np.nan, .5, 1.5, 4.])
    write_csv('interpolation', {'X': x, 'Y': y, 'target_X': target})
    e['F159'] = dict(grid_X=np.linspace(0,4,9).tolist(), grid_Y=(2*np.linspace(0,4,9)+1).tolist(),
                     target_Y=[6., None, 2., 4., 9.], query_X=1.25, query_Y=3.5)
    x1=np.array([0.,2.,4.]); y1=x1.copy(); x2=np.array([1.,2.,5.]); y2=10*x2
    write_csv('curve_merge', {'X1':x1,'Y1':y1,'X2':x2,'Y2':y2})
    grid=np.linspace(1,4,3); values=np.stack([np.interp(grid,x1,y1),np.interp(grid,x2,y2)])
    pairs=sorted(zip(np.r_[x1,x2],np.r_[y1,y2]),key=lambda p:p[0]); unique=np.unique([p[0] for p in pairs])
    e['F167']=dict(average_X=grid.tolist(),average_Y=values.mean(axis=0).tolist(),sample_SD=values.std(axis=0,ddof=1).tolist(),
                  merged_X=unique.tolist(),merged_Y=[float(np.mean([y for xx,y in pairs if xx==v])) for v in unique],
                  keep_all=[[float(x),float(y)] for x,y in pairs])
    x=np.linspace(0,10,501); base=2+.3*x
    y=base+4*np.exp(-.5*((x-3)/.25)**2)+2*np.exp(-.5*((x-7)/.3)**2)+1e-5*np.sin(2*np.pi*x)
    write_csv('baseline_peaks', {'X':x,'signal':y})
    write_csv('baseline_polynomial', {'X':x,'Y':base})
    e['F216']=dict(linear_coefficients=np.polynomial.polynomial.polyfit(x,base,1).tolist(),
                  anchors=[0,1,5,9,10],anchor_residual_tolerance=1e-6,polynomial_tolerance=1e-8)
    x=np.linspace(0,10,501); y=sum(height*np.exp(-.5*((x-center)/.18)**2) for center,height in [(2.,1.),(5.,2.),(8.,3.)])
    y+=1e-6*np.sin(1.7*np.arange(len(x)))
    write_csv('peak_detection',{'X':x,'Y':y})
    peaks,_=signal.find_peaks(y,prominence=.1*np.ptp(y))
    derivative=signal.savgol_filter(y,7,3,deriv=1,delta=.02)
    crossings=np.flatnonzero((derivative[:-1] > 0) & (derivative[1:] <= 0))
    derivative_peaks=[float(x[i]) for i in crossings if y[i] > .1*np.ptp(y)]
    e['F218']=dict(peaks=x[peaks].tolist(),count=len(peaks),derivative_peaks=derivative_peaks,
                  prominence_percent=10,SG_window=7,position_tolerance=.04)
    write_csv('statistics',{'complete':np.arange(1.,11.),'with_missing':[1,2,3,4,None,6,7,8,9,10]})
    def description(values):
        v=np.asarray([x for x in values if x is not None],dtype=float)
        return dict(N=len(v),mean=float(v.mean()),sample_SD=float(v.std(ddof=1)),SE=float(stats.sem(v)),
                    minimum=float(v.min()),maximum=float(v.max()),Q1=float(np.quantile(v,.25)),
                    median=float(np.median(v)),Q3=float(np.quantile(v,.75)),sum=float(v.sum()),
                    shapiro_W=float(stats.shapiro(v).statistic),shapiro_p=float(stats.shapiro(v).pvalue))
    e['statistics']={'complete':description(range(1,11)),'with_missing':description([1,2,3,4,None,6,7,8,9,10])}
    wave=[]
    for source, names in [('builtin_fits',['Sine','SineDamp','Pulse']),('square_wave',['square','square-noisy'])]:
        for c in cases(source):
            if c['name'] not in names: continue
            filename='wave_'+c['name'].lower().replace('-','_');write_csv(filename,{'X':c['x'],'Y':c['y']})
            wave.append(dict(file=filename+'.csv',model=c['model'],params=c['params'],initial=c['extra_initial'],
                             source='crates/scicanvas-core/tests/reference/'+source+'.json',N=len(c['x'])))
    e['F153']=wave
    inhibition=[]
    for c in cases('inhibition_fits'):
        columns={}
        for i,g in enumerate(c['groups']):columns['X'+str(i+1)]=g['x'];columns['Y'+str(i+1)]=g['y']
        name='inhibition_'+c['model'].lower();write_csv(name,columns)
        inhibition.append(dict(file=name+'.csv',model=c['model'],params=c['params'],initial=c['initial']))
    e['F154']=inhibition
    x=np.geomspace(.05,20,81); truth=[8.,.7]
    fn=lambda x,q,k:q*k*x/(1+k*x)
    y=fn(x,*truth)+1e-4*np.sin(1.7*np.arange(len(x)))
    p,_=optimize.curve_fit(fn,x,y,p0=[8.8,.77],maxfev=100000)
    write_csv('binding_langmuir',{'X':x,'Y':y});binding=[dict(file='binding_langmuir.csv',model='Langmuir',params=p.tolist(),N=len(x))]
    for c in cases('builtin_fits'):
        if c['name'] not in ['BET','TwoSiteBind']:continue
        x=list(c['x']);y=list(c['y']);n=len(x)
        if c['model']=='BET':x += [1.,1.2];y += [99.,99.]
        name='binding_'+c['model'].lower();write_csv(name,{'X':x,'Y':y})
        binding.append(dict(file=name+'.csv',model=c['model'],params=c['params'],N=n))
    e['F155']=binding
    c=next(c for c in cases('peak_sums') if c['shape']=='Gauss')
    write_csv('multipeak',{'X':c['x'],'Y':c['y']});e['F140']=dict(params=c['params'],initial=c.get('initial',c['params']),source='crates/scicanvas-core/tests/reference/peak_sums.json')
    x=np.linspace(-5,6,551); truth=np.array([-.3,1.,4.,.3,1.2,3.,3.,.8,2.,.5,.02])
    def peaks_model(x,*p):
        return p[-2]+p[-1]*x+sum(p[3*i+2]/(p[3*i+1]*np.sqrt(np.pi/2))*np.exp(-2*((x-p[3*i])/p[3*i+1])**2) for i in range(3))
    y=peaks_model(x,*truth)+1e-4*np.sin(1.7*np.arange(len(x)))
    initial=truth*1.05;p,_=optimize.curve_fit(peaks_model,x,y,p0=initial,maxfev=100000,ftol=1e-12,xtol=1e-12,gtol=1e-12)
    write_csv('overlapping_peaks',{'X':x,'Y':y});e['F220']=dict(params=p.tolist(),initial=initial.tolist(),truth=truth.tolist())
    axis_values={'positive':[1,2,4,8,16],'probability':[.1,5,50,95,99.9],'logit':[.001,.05,.5,.95,.999],'negative_reciprocal':[-10,-5,-2,-1,-.5]}
    e['F094']={}
    for name, values in axis_values.items():
        v=np.asarray(values,dtype=float);write_csv('axis_'+name,{'X':v,'Y':v})
        t=stats.norm.ppf(v/100) if name=='probability' else np.log(v/(1-v)) if name=='logit' else 1/v if name=='negative_reciprocal' else np.log(v)
        e['F094'][name]=dict(values=values,positions=((t-t[0])/(t[-1]-t[0])).tolist())
    # Numeric observations for the unchanged bundled display examples.
    weights=np.array([2.,3.,4.,3.,2.,1.,2.,3.,2.])
    e['F295']=dict(source='samples/30-饼图.scv',weights=weights.tolist(),total=float(weights.sum()),
                   percent=(100*weights/weights.sum()).tolist(),start='12 oclock',direction='clockwise')
    e['F363']=dict(source='samples/31-小提琴图.scv',
                   B_quantiles=np.quantile(weights,[.25,.5,.75]).tolist(),
                   C_quantiles=np.quantile([1,2,3,4,5,4,3,2,1],[.25,.5,.75]).tolist(),N_each=9)
    angles=np.arange(0.,361.,45.);radians=np.deg2rad(angles)
    e['F413']=dict(sources=['samples/32-极坐标线图.scv','samples/33-极坐标散点图.scv'],
                   angle_degrees=angles.tolist(),B_radius=weights.tolist(),
                   B_cartesian=np.column_stack((weights*np.cos(radians),weights*np.sin(radians))).tolist())
    e['F101']=dict(source='samples/25-颜色映射散点图.scv',point_count=6,color_range=[1,6],
                   viridis_endpoints=[[68,1,84],[253,231,37]])
    e['F269']=dict(source='samples/25-颜色映射散点图.scv',B=[2,4,3,6,4,5],C=[1,2,3,4,5,6],
                   B_color_range=[2,6],same_color_rows=[2,5])
    e['F275']=dict(source='samples/29-线系列图.scv',row_lines=6,legend_items=6,
                   first_row=[2,1],last_row=[5,2])
    e['F085']=dict(query_X=[0.,float(np.pi/2),float(np.pi)],sin=[0.,1.,0.],cos=[1.,0.,-1.],points=101)
    (OUT/'expected.json').write_text(json.dumps(expected,ensure_ascii=False,indent=2,allow_nan=False)+'\n',encoding='utf-8',newline='\n')
    print('Generated',len(list(OUT.glob('*.csv'))),'CSVs and independent expectations in',OUT)


if __name__=='__main__':main()
