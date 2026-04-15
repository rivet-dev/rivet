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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsShare.js
var require_faCreativeCommonsShare = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCreativeCommonsShare.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "creative-commons-share";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f4f2";
    var svgPathData = "M255.6 8C397.4 8 504 118.1 504 256 504 403.1 385.5 504 255.6 504 121.6 504 8 394.5 8 256 8 123.1 112.7 8 255.6 8zm.8 44.7c-118.2 0-203.7 97.9-203.7 203.3 0 109.8 91.2 202.8 203.7 202.8 103.2 0 202.8-81.1 202.8-202.8 .1-113.8-90.2-203.3-202.8-203.3zm101 132.4c7.8 0 13.7 6.1 13.7 13.7l0 182.5c0 7.7-6.1 13.7-13.7 13.7l-135.1 0c-7.7 0-13.7-6-13.7-13.7l0-54-54 0c-7.8 0-13.7-6-13.7-13.7l0-182.5c0-8.2 6.6-12.7 12.4-13.7l136.4 0c7.7 0 13.7 6 13.7 13.7l0 54 54 0zM167.9 300.3l40.7 0 0-101.4c0-7.4 5.8-12.6 12-13.7l55.8 0 0-40.3-108.5 0 0 155.4zm176.2-88.1l-108.5 0 0 155.4 108.5 0 0-155.4z";
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
    exports.faCreativeCommonsShare = exports.definition;
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

// src/_entries/faCreativeCommonsShare.js
var import_faCreativeCommonsShare = __toESM(require_faCreativeCommonsShare());
var export_faCreativeCommonsShare = import_faCreativeCommonsShare.definition;
export {
  export_faCreativeCommonsShare as faCreativeCommonsShare
};
