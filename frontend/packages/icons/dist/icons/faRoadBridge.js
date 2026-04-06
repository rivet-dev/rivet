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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRoadBridge.js
var require_faRoadBridge = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRoadBridge.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "road-bridge";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e563";
    var svgPathData = "M32 32l208 0 0 64-24 0 0 64 24 0 0 129.3c-45.4 7.6-80 47.1-80 94.7l0 64c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-94c0-38.8-26.4-72.6-64-82l0-112 40 0 0-64-8 0C14.3 96 0 81.7 0 64S14.3 32 32 32zM88 96l0 64 80 0 0-64-80 0zM336 32l72 0 0 72c0 13.3 10.7 24 24 24s24-10.7 24-24l0-72 72 0c26.5 0 48 21.5 48 48l0 352c0 26.5-21.5 48-48 48l-72 0 0-72c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 72-72 0c-26.5 0-48-21.5-48-48l0-352c0-26.5 21.5-48 48-48zm96 160c-13.3 0-24 10.7-24 24l0 80c0 13.3 10.7 24 24 24s24-10.7 24-24l0-80c0-13.3-10.7-24-24-24z";
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
    exports.faRoadBridge = exports.definition;
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

// src/_entries/faRoadBridge.js
var import_faRoadBridge = __toESM(require_faRoadBridge());
var export_faRoadBridge = import_faRoadBridge.definition;
export {
  export_faRoadBridge as faRoadBridge
};
