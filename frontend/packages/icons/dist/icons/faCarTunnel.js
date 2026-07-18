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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCarTunnel.js
var require_faCarTunnel = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCarTunnel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-tunnel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4de";
    var svgPathData = "M256 0C114.6 0 0 114.6 0 256L0 448c0 35.3 28.7 64 64 64l42.8 0c-6.6-5.9-10.8-14.4-10.8-24l0-113.1c0-13.5 3.1-26.8 9-38.9l39.7-80.8c9.4-19.1 28.9-31.3 50.2-31.3L317 224c21.3 0 40.8 12.1 50.2 31.3L407 336c5.9 12.1 9 25.4 9 38.9L416 488c0 9.6-4.2 18.1-10.8 24l42.8 0c35.3 0 64-28.7 64-64l0-192C512 114.6 397.4 0 256 0zM362.8 512c-6.6-5.9-10.8-14.4-10.8-24l0-24-192 0 0 24c0 9.6-4.2 18.1-10.8 24l213.7 0zm-175-235.5l-29.3 59.5 194.9 0-29.3-59.5c-1.3-2.7-4.1-4.5-7.2-4.5L195 272c-3 0-5.8 1.7-7.2 4.5zM176 424a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm184-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faCarTunnel = exports.definition;
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

// src/_entries/faCarTunnel.js
var import_faCarTunnel = __toESM(require_faCarTunnel());
var export_faCarTunnel = import_faCarTunnel.definition;
export {
  export_faCarTunnel as faCarTunnel
};
