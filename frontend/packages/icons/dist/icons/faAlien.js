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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlien.js
var require_faAlien = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlien.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "alien";
    var width = 448;
    var height = 512;
    var aliases = [128125];
    var unicode = "f8f5";
    var svgPathData = "M224 0C100.3 0 0 100.3 0 224C0 359.3 148.8 471.7 194.7 503.2c8.6 5.9 18.9 8.8 29.3 8.8s20.7-2.9 29.3-8.8C299.2 471.7 448 359.3 448 224C448 100.3 347.7 0 224 0zm32 306.3c0-45.4 36.8-82.3 82.3-82.3l32 0c7.6 0 13.7 6.1 13.7 13.7c0 45.4-36.8 82.3-82.3 82.3l-32 0c-7.6 0-13.7-6.1-13.7-13.7zM109.7 224c45.4 0 82.3 36.8 82.3 82.3c0 7.6-6.1 13.7-13.7 13.7l-32 0C100.8 320 64 283.2 64 237.7c0-7.6 6.1-13.7 13.7-13.7l32 0z";
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
    exports.faAlien = exports.definition;
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

// src/_entries/faAlien.js
var import_faAlien = __toESM(require_faAlien());
var export_faAlien = import_faAlien.definition;
export {
  export_faAlien as faAlien
};
