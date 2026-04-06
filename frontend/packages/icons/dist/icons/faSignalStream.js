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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalStream.js
var require_faSignalStream = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalStream.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "signal-stream";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8dd";
    var svgPathData = "M108.2 71c13.8 11.1 16 31.2 5 45C82.4 154.4 64 203 64 256s18.4 101.6 49.1 140c11.1 13.8 8.8 33.9-5 45s-33.9 8.8-45-5C23.7 386.7 0 324.1 0 256S23.7 125.3 63.2 76c11.1-13.8 31.2-16 45-5zm359.7 0c13.8-11.1 33.9-8.8 45 5C552.3 125.3 576 187.9 576 256s-23.7 130.7-63.2 180c-11.1 13.8-31.2 16-45 5s-16-31.2-5-45c30.7-38.4 49.1-87 49.1-140s-18.4-101.6-49.1-140c-11.1-13.8-8.8-33.9 5-45zM232 256a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zm-27.5-74.7c-17.8 19.8-28.5 46-28.5 74.7s10.8 54.8 28.5 74.7c11.8 13.2 10.7 33.4-2.5 45.2s-33.4 10.7-45.2-2.5C129 342.2 112 301.1 112 256s17-86.2 44.8-117.3c11.8-13.2 32-14.3 45.2-2.5s14.3 32 2.5 45.2zm214.7-42.7C447 169.8 464 210.9 464 256s-17 86.2-44.8 117.3c-11.8 13.2-32 14.3-45.2 2.5s-14.3-32-2.5-45.2c17.8-19.8 28.5-46 28.5-74.7s-10.8-54.8-28.5-74.7c-11.8-13.2-10.7-33.4 2.5-45.2s33.4-10.7 45.2 2.5z";
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
    exports.faSignalStream = exports.definition;
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

// src/_entries/faSignalStream.js
var import_faSignalStream = __toESM(require_faSignalStream());
var export_faSignalStream = import_faSignalStream.definition;
export {
  export_faSignalStream as faSignalStream
};
