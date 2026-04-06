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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignatureLock.js
var require_faSignatureLock = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignatureLock.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "signature-lock";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3ca";
    var svgPathData = "M96 128c0-17.7 14.3-32 32-32s32 14.3 32 32l0 7.8c0 27.7-2.4 55.3-7.1 82.5L68.4 243.7C27.8 255.8 0 293.2 0 335.6l0 71.9c0 40 32.5 72.5 72.5 72.5c26 0 50-13.9 62.9-36.5l13.9-24.3c26.8-47 46.5-97.7 58.4-150.5l94.4-28.3-12.5 37.5c-3.3 9.8-1.6 20.5 4.4 28.8s15.7 13.3 26 13.3l72.6 0c5.6-9.7 13.7-17.8 23.4-23.4l0-24.6c0-5.4 .4-10.8 1.1-16l-52.7 0 18-53.9c3.8-11.3 .9-23.8-7.4-32.4s-20.7-11.8-32.2-8.4L220.4 198.1c2.4-20.7 3.6-41.4 3.6-62.3l0-7.8c0-53-43-96-96-96s-96 43-96 96l0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32zM209.9 368c-6.7 16.3-14.2 32.3-22.3 48L384 416l0-48-174.1 0zM86.8 305l49-14.7c-10.4 33.8-24.5 66.4-42.1 97.2L79.9 411.7c-1.5 2.6-4.3 4.3-7.4 4.3c-4.7 0-8.5-3.8-8.5-8.5l0-71.9c0-14.1 9.3-26.6 22.8-30.7zM528 240c17.7 0 32 14.3 32 32l0 48-64 0 0-48c0-17.7 14.3-32 32-32zm-80 32l0 48c-17.7 0-32 14.3-32 32l0 128c0 17.7 14.3 32 32 32l160 0c17.7 0 32-14.3 32-32l0-128c0-17.7-14.3-32-32-32l0-48c0-44.2-35.8-80-80-80s-80 35.8-80 80z";
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
    exports.faSignatureLock = exports.definition;
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

// src/_entries/faSignatureLock.js
var import_faSignatureLock = __toESM(require_faSignatureLock());
var export_faSignatureLock = import_faSignatureLock.definition;
export {
  export_faSignatureLock as faSignatureLock
};
