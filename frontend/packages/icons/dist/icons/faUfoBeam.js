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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUfoBeam.js
var require_faUfoBeam = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUfoBeam.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ufo-beam";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e048";
    var svgPathData = "M184 152s24 40 104 40s104-40 104-40c0-48-33.3-104-104-104s-104 56-104 104zm-42.6-40.4C159.1 47.2 218.1 0 288 0s128.9 47.2 146.6 111.6C519.2 131.1 576 167 576 208c0 61.9-128.9 112-288 112S0 269.9 0 208c0-41 56.8-76.9 141.4-96.4zM120 208a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm360 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48zM312 256a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM74.3 325.7l4.2-8.9c14.3 5.6 29.7 10.6 46 15l-6.8 14.5-72 152c-5.7 12-20 17.1-32 11.4s-17.1-20-11.4-32l72-152zm384 20.6l-6.8-14.5c16.3-4.4 31.7-9.4 46-15l4.2 8.9 72 152c5.7 12 .6 26.3-11.4 32s-26.3 .6-32-11.4l-72-152z";
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
    exports.faUfoBeam = exports.definition;
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

// src/_entries/faUfoBeam.js
var import_faUfoBeam = __toESM(require_faUfoBeam());
var export_faUfoBeam = import_faUfoBeam.definition;
export {
  export_faUfoBeam as faUfoBeam
};
