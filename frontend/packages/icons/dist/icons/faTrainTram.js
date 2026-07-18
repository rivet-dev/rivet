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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTrainTram.js
var require_faTrainTram = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTrainTram.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "train-tram";
    var width = 384;
    var height = 512;
    var aliases = [128650];
    var unicode = "e5b4";
    var svgPathData = "M0 8C0-5.3 10.7-16 24-16l336 0c13.3 0 24 10.7 24 24l0 32c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-8-120 0 0 64 40 0c53 0 96 43 96 96l0 160c0 31.2-14.9 59-38 76.5l64.3 76c8.6 10.1 7.3 25.3-2.8 33.8s-25.3 7.3-33.8-2.8l-74.6-88.1c-3.6 .4-7.3 .6-11.1 .6l-128 0c-3.8 0-7.5-.2-11.1-.6L42.3 535.5c-8.6 10.1-23.7 11.4-33.8 2.8s-11.4-23.7-2.8-33.8l64.3-76C46.9 411 32 383.2 32 352l0-160c0-53 43-96 96-96l40 0 0-64-120 0 0 8c0 13.3-10.7 24-24 24S0 53.3 0 40L0 8zM128 160c-17.7 0-32 14.3-32 32l0 32c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32l-128 0zm32 192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm96 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faTrainTram = exports.definition;
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

// src/_entries/faTrainTram.js
var import_faTrainTram = __toESM(require_faTrainTram());
var export_faTrainTram = import_faTrainTram.definition;
export {
  export_faTrainTram as faTrainTram
};
