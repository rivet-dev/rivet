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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWagonCovered.js
var require_faWagonCovered = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWagonCovered.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wagon-covered";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8ee";
    var svgPathData = "M4.5 57.4C-5.2 30.3 16.1 2.1 44.8 4.1L194.1 14C210.9 15.1 224 29.1 224 45.9L224 224 64 224 4.5 57.4zM288 16l64 0c17.7 0 32 14.3 32 32l0 176-128 0 0-176c0-17.7 14.3-32 32-32zM64 288c0-17.7 14.3-32 32-32l448 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-1.6 0c20.7 20.3 33.6 48.7 33.6 80c0 61.9-50.1 112-112 112c-56.4 0-103.1-41.7-110.9-96l-66.3 0c-7.8 54.3-54.4 96-110.9 96c-61.9 0-112-50.1-112-112c0-31.3 12.9-59.7 33.6-80L96 320c-17.7 0-32-14.3-32-32zm96 128l-46 0c5.8 22.5 23.5 40.2 46 46l0-46zm0-32l0-46c-22.5 5.8-40.2 23.5-46 46l46 0zm32 78c22.5-5.8 40.2-23.5 46-46l-46 0 0 46zm0-124l0 46 46 0c-5.8-22.5-23.5-40.2-46-46zM448 462l0-46-46 0c5.8 22.5 23.5 40.2 46 46zm-46-78l46 0 0-46c-22.5 5.8-40.2 23.5-46 46zm124 32l-46 0 0 46c22.5-5.8 40.2-23.5 46-46zm0-32c-5.8-22.5-23.5-40.2-46-46l0 46 46 0zM595.2 4.1c28.7-1.9 50 26.2 40.3 53.4L576 224l-160 0 0-178.1c0-16.8 13.1-30.8 29.9-31.9l149.3-10z";
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
    exports.faWagonCovered = exports.definition;
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

// src/_entries/faWagonCovered.js
var import_faWagonCovered = __toESM(require_faWagonCovered());
var export_faWagonCovered = import_faWagonCovered.definition;
export {
  export_faWagonCovered as faWagonCovered
};
