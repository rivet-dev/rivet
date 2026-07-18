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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsNd.js
var require_faCreativeCommonsNd = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsNd.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "creative-commons-nd";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4eb";
    var svgPathData = "M255.6 8C397.4 8 504 118.1 504 256 504 403.1 385.5 504 255.6 504 121.6 504 8 394.5 8 256 8 123.1 112.7 8 255.6 8zm.8 44.7c-118.2 0-203.7 97.9-203.7 203.3 0 109.8 91.2 202.8 203.7 202.8 103.2 0 202.8-81.1 202.8-202.8 .1-113.8-90.2-203.3-202.8-203.3zm94 144.3l0 42.5-180.3 0 0-42.5 180.3 0zm0 79.8l0 42.5-180.3 0 0-42.5 180.3 0z";
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
    exports.faCreativeCommonsNd = exports.definition;
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

// src/_entries/faCreativeCommonsNd.js
var import_faCreativeCommonsNd = __toESM(require_faCreativeCommonsNd());
var export_faCreativeCommonsNd = import_faCreativeCommonsNd.definition;
export {
  export_faCreativeCommonsNd as faCreativeCommonsNd
};
