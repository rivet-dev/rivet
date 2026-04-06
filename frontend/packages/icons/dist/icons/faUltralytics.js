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

// src/node_modules/@fortawesome/free-brands-svg-icons/faUltralytics.js
var require_faUltralytics = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faUltralytics.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ultralytics";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e86d";
    var svgPathData = "M391.4 39.7c65.8-.2 118.6 51.8 120.4 116.6 .7 26.4-1.5 54.1-3.8 74.1-17.2 148.6-122.4 242-252.4 242-95.4-.1-179.4-52.7-223.2-130.6 24.4 17.4 54.5 27.4 87.6 27.4 83.2 .1 151.4-67.4 151.2-150l.1-60.6c-.2-65.7 53.9-119 120.1-118.9zM119.9 100.2c66.2 0 119.9 53.3 119.9 119S186.1 338.3 119.9 338.3 0 285 0 219.3c0-65.7 53.7-119 119.9-119z";
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
    exports.faUltralytics = exports.definition;
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

// src/_entries/faUltralytics.js
var import_faUltralytics = __toESM(require_faUltralytics());
var export_faUltralytics = import_faUltralytics.definition;
export {
  export_faUltralytics as faUltralytics
};
