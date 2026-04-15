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

// src/node_modules/@fortawesome/free-brands-svg-icons/faGooglePlus.js
var require_faGooglePlus = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faGooglePlus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "google-plus";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f2b3";
    var svgPathData = "M256 8a248 248 0 1 0 0 496 248 248 0 1 0 0-496zM185.3 380c-32.9 0-64.4-13.1-87.7-36.3s-36.3-54.8-36.3-87.7 13.1-64.4 36.3-87.7 54.8-36.3 87.7-36.3c31.3 0 60.1 11 83 32.3l-33.6 32.6c-13.2-12.9-31.3-19.1-49.4-19.1-42.9 0-77.2 35.5-77.2 78.1S142.3 334 185.3 334c32.6 0 64.9-19.1 70.1-53.3l-70.1 0 0-42.6 116.9 0c1.3 6.8 1.9 13.8 1.9 20.7 0 70.8-47.5 121.2-118.8 121.2zM415.5 273.8l0 35.5-35.5 0 0-35.5-35.5 0 0-35.5 35.5 0 0-35.5 35.5 0 0 35.5 35.2 0 0 35.5-35.2 0z";
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
    exports.faGooglePlus = exports.definition;
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

// src/_entries/faGooglePlus.js
var import_faGooglePlus = __toESM(require_faGooglePlus());
var export_faGooglePlus = import_faGooglePlus.definition;
export {
  export_faGooglePlus as faGooglePlus
};
