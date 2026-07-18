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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarBuilding.js
var require_faCarBuilding = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarBuilding.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-building";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f859";
    var svgPathData = "M48 0C21.5 0 0 21.5 0 48L0 432c0 26.5 21.5 48 48 48l144 0 0-32 0-96c0-32.8 16.5-61.8 41.6-79.1l5.6-16.9L208 256c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16l32 0c8.2 0 14.9 6.1 15.9 14l.5-1.4c10.5-31.6 34.2-56 63.7-68.2L320 48c0-26.5-21.5-48-48-48L48 0zM80 192l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zm0-96l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zm112 16c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32zM80 288l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zm282.6-64l138.8 0c6.9 0 13 4.4 15.2 10.9L534.3 288l-204.5 0 17.7-53.1c2.2-6.5 8.3-10.9 15.2-10.9zm-75.9-9.3l-26.6 79.7C238.8 304.7 224 326.6 224 352l0 96s0 0 0 0l0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 256 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 0-96c0-25.4-14.8-47.3-36.1-57.6l-26.6-79.7C566.4 182 535.8 160 501.4 160l-138.8 0c-34.4 0-65 22-75.9 54.7zM304 344a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm232 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faCarBuilding = exports.definition;
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

// src/_entries/faCarBuilding.js
var import_faCarBuilding = __toESM(require_faCarBuilding());
var export_faCarBuilding = import_faCarBuilding.definition;
export {
  export_faCarBuilding as faCarBuilding
};
