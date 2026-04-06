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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVanShuttle.js
var require_faVanShuttle = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVanShuttle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "van-shuttle";
    var width = 576;
    var height = 512;
    var aliases = [128656, "shuttle-van"];
    var unicode = "f5b6";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 336c0 35.3 28.7 64 64 64l.4 0c4 44.9 41.7 80 87.6 80s83.6-35.1 87.6-80l104.7 0c4 44.9 41.7 80 87.6 80 46.1 0 83.9-35.4 87.7-80.5 31.7-3.8 56.3-30.8 56.3-63.5l0-101.3c0-13.8-4.5-27.3-12.8-38.4l-80-106.7C471.1 73.5 452.1 64 432 64L64 64zM504 224l-120 0 0-96 48 0 72 96zM64 224l0-96 96 0 0 96-96 0zm160 0l0-96 96 0 0 96-96 0zM392 392a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zM152 352a40 40 0 1 1 0 80 40 40 0 1 1 0-80z";
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
    exports.faVanShuttle = exports.definition;
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

// src/_entries/faVanShuttle.js
var import_faVanShuttle = __toESM(require_faVanShuttle());
var export_faVanShuttle = import_faVanShuttle.definition;
export {
  export_faVanShuttle as faVanShuttle
};
