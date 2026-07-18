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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneUp.js
var require_faPlaneUp = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-up";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e22d";
    var svgPathData = "M200 24c0-30.9 25.1-56 56-56s56 25.1 56 56l0 127.3 173.6 159.2c6.6 6.1 10.4 14.6 10.4 23.6l0 43.7c0 10.9-10.7 18.6-21.1 15.2l-162.9-54.3 0 99.7 66 52.8c3.8 3 6 7.6 6 12.5l0 19.8c0 10.4-9.8 18-19.9 15.5L256 512 147.9 539c-10.1 2.5-19.9-5.1-19.9-15.5l0-19.8c0-4.9 2.2-9.5 6-12.5l66-52.8 0-99.7-162.9 54.3C26.7 396.4 16 388.7 16 377.8l0-43.7c0-9 3.8-17.5 10.4-23.6L200 151.3 200 24z";
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
    exports.faPlaneUp = exports.definition;
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

// src/_entries/faPlaneUp.js
var import_faPlaneUp = __toESM(require_faPlaneUp());
var export_faPlaneUp = import_faPlaneUp.definition;
export {
  export_faPlaneUp as faPlaneUp
};
