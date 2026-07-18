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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGalaxy.js
var require_faGalaxy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGalaxy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "galaxy";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e008";
    var svgPathData = "M320 0l11.5 0C342.8 0 352 9.2 352 20.5c0 7.1-3.7 13.7-9.8 17.5l-12.5 7.7c-35.2 21.7-60.9 54.9-73.3 93.1c19.9-7 41.3-10.8 63.5-10.8c106 0 192 86 192 192l0 11.5c0 11.3-9.2 20.5-20.5 20.5c-7.1 0-13.7-3.7-17.5-9.8l-7.7-12.5c-21.7-35.2-54.9-60.9-93.1-73.3c7 19.9 10.8 41.3 10.8 63.5c0 106-86 192-192 192l-11.5 0c-11.3 0-20.5-9.2-20.5-20.5c0-7.1 3.7-13.7 9.8-17.5l12.5-7.7c35.2-21.7 60.9-54.9 73.3-93.1c-19.9 7-41.3 10.8-63.5 10.8C86 384 0 298 0 192l0-11.5C0 169.2 9.2 160 20.5 160c7.1 0 13.7 3.7 17.5 9.8l7.7 12.5c21.7 35.2 54.9 60.9 93.1 73.3c-7-19.9-10.8-41.3-10.8-63.5C128 86 214 0 320 0zM256 288a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faGalaxy = exports.definition;
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

// src/_entries/faGalaxy.js
var import_faGalaxy = __toESM(require_faGalaxy());
var export_faGalaxy = import_faGalaxy.definition;
export {
  export_faGalaxy as faGalaxy
};
