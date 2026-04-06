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

// src/node_modules/@fortawesome/free-brands-svg-icons/faWirsindhandwerk.js
var require_faWirsindhandwerk = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faWirsindhandwerk.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "wirsindhandwerk";
    var width = 448;
    var height = 512;
    var aliases = ["wsh"];
    var unicode = "e2d0";
    var svgPathData = "M18.9 479.8l83.4 0 0-112-83.4 47 0 65zm329 0l82.4 0 0-65-82.4-47 0 112zm0-448l0 219.8-123.6-72.4-121.7 72.4 0-219.8-83.7 0 0 360.8 205.4-122.3 206 122.3 0-360.8-82.3 0z";
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
    exports.faWirsindhandwerk = exports.definition;
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

// src/_entries/faWsh.js
var import_faWirsindhandwerk = __toESM(require_faWirsindhandwerk());
var export_faWsh = import_faWirsindhandwerk.definition;
export {
  export_faWsh as faWsh
};
