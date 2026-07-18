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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKnifeKitchen.js
var require_faKnifeKitchen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKnifeKitchen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "knife-kitchen";
    var width = 576;
    var height = 512;
    var aliases = [128298];
    var unicode = "f6f5";
    var svgPathData = "M488 0c9.4 0 18.3 3.7 25 10.3L557.7 55c6.6 6.6 10.3 15.6 10.3 25s-3.7 18.3-10.3 25l-97 97c-3 3-4.7 7.1-4.7 11.3l0 31.9c0 17.4-9.4 33.5-24.7 42l-31 17.2L295.9 200.1l-11.3-11.3L463 10.3C469.7 3.7 478.6 0 488 0zM432 160a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm80-80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM262 211.4l11.3 11.3 126 126c6.2 6.2 6.2 16.4 0 22.6C321.4 449.2 235.3 512 112 512c-25.6 0-50.9-4.2-69.5-8.4c-9.4-2.1-17.2-4.2-22.7-5.7c-2.8-.8-4.9-1.4-6.5-1.9c-.8-.2-1.4-.4-1.8-.6l-.5-.2-.1 0c0 0 0 0 0 0s0 0 0 0s0 0 0 0L16 480l-5.1 15.2c-5.3-1.8-9.2-6.1-10.5-11.5s.3-11.1 4.3-15l246-246L262 211.4z";
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
    exports.faKnifeKitchen = exports.definition;
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

// src/_entries/faKnifeKitchen.js
var import_faKnifeKitchen = __toESM(require_faKnifeKitchen());
var export_faKnifeKitchen = import_faKnifeKitchen.definition;
export {
  export_faKnifeKitchen as faKnifeKitchen
};
