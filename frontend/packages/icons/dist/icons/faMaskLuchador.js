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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLuchadorMask.js
var require_faLuchadorMask = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLuchadorMask.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "luchador-mask";
    var width = 448;
    var height = 512;
    var aliases = ["luchador", "mask-luchador"];
    var unicode = "f455";
    var svgPathData = "M224 0C100.3 0 0 100.3 0 224l0 96C0 426 86 512 192 512l64 0c106 0 192-86 192-192l0-96C448 100.3 347.7 0 224 0zM128 384c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-35.3 0-57.4 0L160 352c-17.7 0-32 14.3-32 32zm-32 0c0-33.2 25.2-60.5 57.6-63.7c-1-3.7-1.6-7.6-1.6-11.6c0-7.3 1.8-14.4 5.3-20.7L144 288c-44.2 0-80-35.8-80-80c0-26.5 21.5-48 48-48l32 0c44.2 0 80 35.8 80 80c0-44.2 35.8-80 80-80l32 0c26.5 0 48 21.5 48 48c0 44.2-35.8 80-80 80l-13.3 0c3.4 6.3 5.3 13.4 5.3 20.7c0 4-.6 7.9-1.6 11.6c32.3 3.2 57.6 30.5 57.6 63.7c0 35.3-28.7 64-64 64l-128 0c-35.3 0-64-28.7-64-64zm156.7-64l.2 0c6.1 0 11.1-5 11.1-11.1l0-.1 0-.1c0-3-1.2-5.9-3.3-8l-8-8-4.2-4.2-.4-.4-10.3-10.3C234 274 229.1 272 224 272s-10 2-13.7 5.7L200 288l-.4 .4-4.2 4.2-8 8c-2.1 2.1-3.3 5-3.3 8l0 .1 0 .1c0 6.1 5 11.1 11.1 11.1l.2 0 11.3 0 6 0 .6 0 21.5 0 .6 0 6 0 11.3 0zM112 192c-8.8 0-16 7.2-16 16c0 26.5 21.5 48 48 48l32 0c8.8 0 16-7.2 16-16c0-26.5-21.5-48-48-48l-32 0zm240 16c0-8.8-7.2-16-16-16l-32 0c-26.5 0-48 21.5-48 48c0 8.8 7.2 16 16 16l32 0c26.5 0 48-21.5 48-48z";
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
    exports.faLuchadorMask = exports.definition;
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

// src/_entries/faMaskLuchador.js
var import_faLuchadorMask = __toESM(require_faLuchadorMask());
var export_faMaskLuchador = import_faLuchadorMask.definition;
export {
  export_faMaskLuchador as faMaskLuchador
};
