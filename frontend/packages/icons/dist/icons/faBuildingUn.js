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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingUn.js
var require_faBuildingUn = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingUn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "building-un";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4d9";
    var svgPathData = "M32 64C32 28.7 60.7 0 96 0L352 0c35.3 0 64 28.7 64 64l0 272-112 0c-20.9 0-39.5 10.1-51.2 25.6-8-6-18-9.6-28.8-9.6-26.5 0-48 21.5-48 48l0 64 64 0 0 48-144 0c-35.3 0-64-28.7-64-64L32 64zM144 96c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0zm112 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zM144 224c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0zm112 16l0 32c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-32c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16zM427.4 380.5c9-2.1 18.3 2.2 22.5 10.5l26.1 52.2 0-43.3c0-11 9-20 20-20s20 9 20 20l0 128c0 9.3-6.4 17.3-15.4 19.5s-18.3-2.2-22.5-10.5L452 484.7 452 528c0 11-9 20-20 20s-20-9-20-20l0-128c0-9.3 6.4-17.3 15.4-19.5zM324 400l0 96c0 6.6 5.4 12 12 12s12-5.4 12-12l0-96c0-11 9-20 20-20s20 9 20 20l0 96c0 28.7-23.3 52-52 52s-52-23.3-52-52l0-96c0-11 9-20 20-20s20 9 20 20z";
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
    exports.faBuildingUn = exports.definition;
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

// src/_entries/faBuildingUn.js
var import_faBuildingUn = __toESM(require_faBuildingUn());
var export_faBuildingUn = import_faBuildingUn.definition;
export {
  export_faBuildingUn as faBuildingUn
};
