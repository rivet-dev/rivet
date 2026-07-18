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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faOliveBranch.js
var require_faOliveBranch = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faOliveBranch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "olive-branch";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e317";
    var svgPathData = "M2.7 53.1C13.8 41.6 59 0 128 0c66.9 0 111.4 39.1 124.2 52L306 26.7C343.3 9.1 384 0 425.2 0L616 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L425.2 48c-34.2 0-67.9 7.5-98.8 22.1L191.9 133.4C189.1 219.3 118.6 288 32 288l-16 0c-8.8 0-16-7.2-16-16l0-16c0-61.4 34.5-114.6 85.2-141.5C40.4 102.8 11.3 75.8 2.7 66.9C.9 65 0 62.6 0 60s.9-5 2.7-6.9zM480 320c0 106-64.5 192-144 192s-144-86-144-192s64.5-192 144-192s144 86 144 192zM465.7 471.8c29.2-40.5 46.3-94 46.3-151.8c0-59.3-18-114.2-48.6-155c-10.5-14-22.7-26.5-36.2-36.9c48.2 1.6 101.3 25.8 144.2 70.3c75 77.8 90.2 188.1 33.9 246.4c-34.1 35.4-86.8 43.5-139.6 27z";
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
    exports.faOliveBranch = exports.definition;
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

// src/_entries/faOliveBranch.js
var import_faOliveBranch = __toESM(require_faOliveBranch());
var export_faOliveBranch = import_faOliveBranch.definition;
export {
  export_faOliveBranch as faOliveBranch
};
