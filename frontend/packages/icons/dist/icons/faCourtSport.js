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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCourtSport.js
var require_faCourtSport = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCourtSport.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "court-sport";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e643";
    var svgPathData = "M576 480l-240 0 0-113.1c54.3-7.8 96-54.4 96-110.9s-41.7-103.1-96-110.9L336 32l240 0c35.3 0 64 28.7 64 64l0 32-64 0c-35.3 0-64 28.7-64 64l0 128c0 35.3 28.7 64 64 64l64 0 0 32c0 35.3-28.7 64-64 64zM304 145.1c-54.3 7.8-96 54.4-96 110.9s41.7 103.1 96 110.9L304 480 64 480c-35.3 0-64-28.7-64-64l0-32 64 0c35.3 0 64-28.7 64-64l0-128c0-35.3-28.7-64-64-64L0 128 0 96C0 60.7 28.7 32 64 32l240 0 0 113.1zm0 189.3c-36.5-7.4-64-39.7-64-78.4s27.5-71 64-78.4l0 156.8zm32 0l0-156.8c36.5 7.4 64 39.7 64 78.4s-27.5 71-64 78.4zM576 160l64 0 0 192-64 0c-17.7 0-32-14.3-32-32l0-128c0-17.7 14.3-32 32-32zM64 352L0 352 0 160l64 0c17.7 0 32 14.3 32 32l0 128c0 17.7-14.3 32-32 32z";
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
    exports.faCourtSport = exports.definition;
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

// src/_entries/faCourtSport.js
var import_faCourtSport = __toESM(require_faCourtSport());
var export_faCourtSport = import_faCourtSport.definition;
export {
  export_faCourtSport as faCourtSport
};
