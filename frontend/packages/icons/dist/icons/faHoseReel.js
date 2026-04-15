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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHoseReel.js
var require_faHoseReel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHoseReel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hose-reel";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e41a";
    var svgPathData = "M320 64c-106 0-192 86-192 192s86 192 192 192c40.8 0 78.6-12.7 109.7-34.4c14.5-10.1 34.4-6.6 44.6 7.9s6.6 34.4-7.9 44.6C424.8 495 374.3 512 320 512C178.6 512 64 397.4 64 256S178.6 0 320 0S576 114.6 576 256l0 96 8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-2.3 0-11.5 80.2c7.8 1.1 13.7 7.8 13.7 15.8c0 8.8-7.2 16-16 16l-48 0c-8.8 0-16-7.2-16-16c0-8.1 6-14.7 13.7-15.8L506.3 400l-2.3 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l8 0 0-96c0-106-86-192-192-192zM224 256a96 96 0 1 0 192 0 96 96 0 1 0 -192 0zm96 160a160 160 0 1 1 0-320 160 160 0 1 1 0 320zM256 256a64 64 0 1 1 128 0 64 64 0 1 1 -128 0z";
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
    exports.faHoseReel = exports.definition;
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

// src/_entries/faHoseReel.js
var import_faHoseReel = __toESM(require_faHoseReel());
var export_faHoseReel = import_faHoseReel.definition;
export {
  export_faHoseReel as faHoseReel
};
