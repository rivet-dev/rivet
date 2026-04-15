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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrainSubwayTunnel.js
var require_faTrainSubwayTunnel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrainSubwayTunnel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "train-subway-tunnel";
    var width = 512;
    var height = 512;
    var aliases = ["subway-tunnel"];
    var unicode = "e2a3";
    var svgPathData = "M256 0C114.6 0 0 114.6 0 256L0 448c0 35.3 28.7 64 64 64l41.4 0 64.3-64.3C137.3 444.5 112 417.2 112 384l0-192c0-35.3 28.7-64 64-64l160 0c35.3 0 64 28.7 64 64l0 192c0 33.2-25.3 60.5-57.7 63.7L406.6 512l41.4 0c35.3 0 64-28.7 64-64l0-192C512 114.6 397.4 0 256 0zM361.4 512l-64-64-82.7 0-64 64 210.7 0zM184 192c-13.3 0-24 10.7-24 24l0 8 0 64 0 8c0 13.3 10.7 24 24 24l8 0 16 0 8 0c13.3 0 24-10.7 24-24l0-8 0-64 0-8c0-13.3-10.7-24-24-24l-8 0-16 0-8 0zm88 24l0 8 0 64 0 8c0 13.3 10.7 24 24 24l8 0 16 0 8 0c13.3 0 24-10.7 24-24l0-8 0-64 0-8c0-13.3-10.7-24-24-24l-8 0-16 0-8 0c-13.3 0-24 10.7-24 24zM184 400a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm168-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faTrainSubwayTunnel = exports.definition;
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

// src/_entries/faTrainSubwayTunnel.js
var import_faTrainSubwayTunnel = __toESM(require_faTrainSubwayTunnel());
var export_faTrainSubwayTunnel = import_faTrainSubwayTunnel.definition;
export {
  export_faTrainSubwayTunnel as faTrainSubwayTunnel
};
