import sys, json, time
t=time.time()
from madmom.features.chords import CNNChordFeatureProcessor, CRFChordRecognitionProcessor
out = CRFChordRecognitionProcessor()(CNNChordFeatureProcessor()(sys.argv[1]))
print(json.dumps({"chords":[[float(a),float(b),str(c)] for a,b,c in out], "sec": round(time.time()-t,2)}))
