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

// src/node_modules/@fortawesome/free-solid-svg-icons/faEarthOceania.js
var require_faEarthOceania = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faEarthOceania.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "earth-oceania";
    var width = 512;
    var height = 512;
    var aliases = ["globe-oceania"];
    var unicode = "e47b";
    var svgPathData = "M256 512a256 256 0 1 0 0-512 256 256 0 1 0 0 512zM192 150.6c0-12.5 10.1-22.6 22.6-22.6l41.4 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-18.7 0c-8.5 0-16.6-3.4-22.6-9.4l-16-16c-4.2-4.2-6.6-10-6.6-16zM432 256c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zM411.3 379.3l-32 32c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6l32-32c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6zM224 432c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16zm-96-80c-17.7 0-32-14.3-32-32l0-18.7c0-8.5 3.4-16.6 9.4-22.6l48-48c4.2-4.2 10-6.6 16-6.6l6.6 0c8.8 0 16 7.2 16 16s7.2 16 16 16 16-7.2 16-16 7.2-16 16-16l6.6 0c6 0 11.8 2.4 16 6.6l48 48c6 6 9.4 14.1 9.4 22.6l0 50.7c0 17.7-14.3 32-32 32l-50.7 0c-8.5 0-16.6-3.4-22.6-9.4l-13.3-13.3c-6-6-14.1-9.4-22.6-9.4L128 352z";
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
    exports.faEarthOceania = exports.definition;
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

// src/_entries/faGlobeOceania.js
var import_faEarthOceania = __toESM(require_faEarthOceania());
var export_faGlobeOceania = import_faEarthOceania.definition;
export {
  export_faGlobeOceania as faGlobeOceania
};
