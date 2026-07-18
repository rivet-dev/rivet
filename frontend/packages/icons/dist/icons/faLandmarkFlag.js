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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLandmarkFlag.js
var require_faLandmarkFlag = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLandmarkFlag.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "landmark-flag";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e51c";
    var svgPathData = "M352-32L240-32c-8.8 0-16 7.2-16 16l0 144-176 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l16 0 0 224-51.2 38.4C4.7 460.4 0 469.9 0 480 0 497.7 14.3 512 32 512l448 0c17.7 0 32-14.3 32-32 0-10.1-4.7-19.6-12.8-25.6l-51.2-38.4 0-224 16 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-192 0 0-64 80 0c8.8 0 16-7.2 16-16l0-64c0-8.8-7.2-16-16-16zm48 224l0 224-64 0 0-224 64 0zm-112 0l0 224-64 0 0-224 64 0zm-112 0l0 224-64 0 0-224 64 0z";
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
    exports.faLandmarkFlag = exports.definition;
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

// src/_entries/faLandmarkFlag.js
var import_faLandmarkFlag = __toESM(require_faLandmarkFlag());
var export_faLandmarkFlag = import_faLandmarkFlag.definition;
export {
  export_faLandmarkFlag as faLandmarkFlag
};
