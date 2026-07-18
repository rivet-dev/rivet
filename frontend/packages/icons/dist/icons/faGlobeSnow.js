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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobeSnow.js
var require_faGlobeSnow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobeSnow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "globe-snow";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7a3";
    var svgPathData = "M448 224c0 62.7-25.7 119.3-67.2 160L256 384l0-32 64 0c6.5 0 12.3-3.9 14.8-9.9s1.1-12.9-3.5-17.4L262.6 256l25.4 0c6.5 0 12.3-3.9 14.8-9.9s1.1-12.9-3.5-17.4l-64-64c-6.2-6.2-16.4-6.2-22.6 0l-64 64c-4.6 4.6-5.9 11.5-3.5 17.4s8.3 9.9 14.8 9.9l25.4 0-68.7 68.7c-4.6 4.6-5.9 11.5-3.5 17.4s8.3 9.9 14.8 9.9l64 0 0 32L67.2 384C25.7 343.3 0 286.7 0 224C0 100.3 100.3 0 224 0S448 100.3 448 224zM256 80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm80 112a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM96 208a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM20.3 474.2L61.1 416l325.8 0 40.8 58.2c5.1 7.3 5.8 16.9 1.6 24.8S416.9 512 408 512L40 512c-8.9 0-17.2-5-21.3-12.9s-3.5-17.5 1.6-24.8z";
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
    exports.faGlobeSnow = exports.definition;
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

// src/_entries/faGlobeSnow.js
var import_faGlobeSnow = __toESM(require_faGlobeSnow());
var export_faGlobeSnow = import_faGlobeSnow.definition;
export {
  export_faGlobeSnow as faGlobeSnow
};
