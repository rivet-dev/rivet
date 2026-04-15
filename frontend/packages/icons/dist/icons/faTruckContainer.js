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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckContainer.js
var require_faTruckContainer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckContainer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-container";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f4dc";
    var svgPathData = "M48 32l304 0c26.5 0 48 21.5 48 48l0 160c0 26.5-21.5 48-48 48L48 288c-26.5 0-48-21.5-48-48L0 80C0 53.5 21.5 32 48 32zM80 80c-8.8 0-16 7.2-16 16l0 128c0 8.8 7.2 16 16 16s16-7.2 16-16L96 96c0-8.8-7.2-16-16-16zM224 96l0 128c0 8.8 7.2 16 16 16s16-7.2 16-16l0-128c0-8.8-7.2-16-16-16s-16 7.2-16 16zM160 80c-8.8 0-16 7.2-16 16l0 128c0 8.8 7.2 16 16 16s16-7.2 16-16l0-128c0-8.8-7.2-16-16-16zM304 96l0 128c0 8.8 7.2 16 16 16s16-7.2 16-16l0-128c0-8.8-7.2-16-16-16s-16 7.2-16 16zm128 72c0-13.3 10.7-24 24-24l72.8 0c16.8 0 32.7 7.5 43.3 20.5L631 236.4c5.8 7.1 9 16.1 9 25.3l0 10.3 0 16 0 64c0 17.7-14.3 32-32 32l-1.6 0c1 5.2 1.6 10.5 1.6 16c0 44.2-35.8 80-80 80s-80-35.8-80-80c0-5.5 .6-10.8 1.6-16L432 384l-145.6 0c1 5.2 1.6 10.5 1.6 16c0 44.2-35.8 80-80 80c-26.2 0-49.4-12.6-64-32c-14.6 19.4-37.8 32-64 32c-44.2 0-80-35.8-80-80l0-16 0-32c0-17.7 14.3-32 32-32l48 0 128 0 224 0 0-32 0-32 0-88zm48 88l105 0-50-61.1c-1.5-1.9-3.8-2.9-6.2-2.9L480 192l0 64zm80 144a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM208 432a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm-96-32a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faTruckContainer = exports.definition;
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

// src/_entries/faTruckContainer.js
var import_faTruckContainer = __toESM(require_faTruckContainer());
var export_faTruckContainer = import_faTruckContainer.definition;
export {
  export_faTruckContainer as faTruckContainer
};
