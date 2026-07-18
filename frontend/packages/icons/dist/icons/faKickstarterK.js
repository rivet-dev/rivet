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

// src/node_modules/@fortawesome/free-brands-svg-icons/faKickstarterK.js
var require_faKickstarterK = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faKickstarterK.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "kickstarter-k";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f3bc";
    var svgPathData = "M356.7 256.2l40.8-40.5c42.2-41.9 42.2-110.3 0-152.1s-111-41.9-153.2 0L229.3 78.4C209.6 50.3 177.1 32 140.2 32 80.5 32 32 80.2 32 139.5l0 233c0 59.4 48.5 107.5 108.2 107.5 37.1 0 69.3-18.3 89-46.4l14.9 14.7c42.2 41.9 111 41.9 153.2 0s42.2-110.3 0-152.1l-40.8-40 .1 0z";
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
    exports.faKickstarterK = exports.definition;
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

// src/_entries/faKickstarterK.js
var import_faKickstarterK = __toESM(require_faKickstarterK());
var export_faKickstarterK = import_faKickstarterK.definition;
export {
  export_faKickstarterK as faKickstarterK
};
