var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMonocle.js
var require_faFaceMonocle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMonocle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-monocle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e380";
    var svgPathData = "M427.2 446.3C381.9 487.2 321.8 512 256 512C114.6 512 0 397.4 0 256S114.6 0 256 0S512 114.6 512 256c0 62.8-22.6 120.3-60.1 164.9c-2.5-8.6-3.9-17.7-3.9-26.9L448 224c0-61.9-50.1-112-112-112s-112 50.1-112 112s50.1 112 112 112c31.3 0 59.7-12.9 80-33.6l0 91.5c0 18.4 3.9 36.2 11.2 52.4zM256 224a80 80 0 1 1 160 0 80 80 0 1 1 -160 0zm-79.6-32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm144 64a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM144 352c0 8.8 7.2 16 16 16c14.3 0 37.9 2.9 60.6 11.7c22.8 8.8 42.7 22.7 53.1 43.4c4 7.9 13.6 11.1 21.5 7.2s11.1-13.6 7.2-21.5c-15.3-30.5-43.3-48.6-70.1-59C205.3 339.5 177.7 336 160 336c-8.8 0-16 7.2-16 16zM305.2 89.9c8-6.4 18-9.9 28.2-9.9c8.9 0 17.6 2.6 25.1 7.6l8.6 5.7c7.4 4.9 17.3 2.9 22.2-4.4s2.9-17.3-4.4-22.2L376.3 61c-12.7-8.5-27.6-13-42.8-13c-17.5 0-34.5 6-48.2 16.9L262 83.5c-6.9 5.5-8 15.6-2.5 22.5s15.6 8 22.5 2.5l23.2-18.6z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faFaceMonocle = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faFaceMonocle.js
var import_faFaceMonocle = __toESM(require_faFaceMonocle());
var export_faFaceMonocle = import_faFaceMonocle.definition;
export {
  export_faFaceMonocle as faFaceMonocle
};
