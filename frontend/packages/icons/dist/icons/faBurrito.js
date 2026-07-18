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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBurrito.js
var require_faBurrito = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBurrito.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "burrito";
    var width = 512;
    var height = 512;
    var aliases = [127791];
    var unicode = "f7ed";
    var svgPathData = "M251.4 52.6L33.9 270.1c-9.9 9.9-17.8 21.3-23.4 33.7C39.2 324.1 74.2 336 112 336c79.2 0 146.2-52.3 168.3-124.3c5-16.3 7.7-33.7 7.7-51.7c0-40.4-13.6-77.7-36.6-107.4zm15-32C299.7 57.5 320 106.4 320 160c0 11.9-1 23.6-2.9 34.9c11.4-1.9 23-2.9 34.9-2.9c53.6 0 102.5 20.3 139.4 53.6c12.8-14.2 20.6-33 20.6-53.6c0-11.4-2.4-22.2-6.7-32c4.3-9.8 6.7-20.6 6.7-32c0-34.6-22-64.1-52.8-75.2C448.1 22 418.6 0 384 0c-11.4 0-22.2 2.4-32 6.7C342.2 2.4 331.4 0 320 0c-20.6 0-39.4 7.8-53.6 20.6zm193 240C429.7 237.6 392.4 224 352 224c-15.2 0-29.9 1.9-43.9 5.5C279.5 310.2 202.5 368 112 368c-40.8 0-78.8-11.7-110.9-32C.4 341.3 0 346.6 0 352c0 30.7 12.2 60.2 33.9 81.9l44.1 44.1C99.8 499.8 129.3 512 160 512s60.2-12.2 81.9-33.9L459.4 260.6z";
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
    exports.faBurrito = exports.definition;
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

// src/_entries/faBurrito.js
var import_faBurrito = __toESM(require_faBurrito());
var export_faBurrito = import_faBurrito.definition;
export {
  export_faBurrito as faBurrito
};
